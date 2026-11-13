//! Files attached to issues. The database stores each file's name, type, and size.
//! The contents are stored in the `uploads/` directory, in a file named after the
//! attachment's ID.

use std::{fmt::Write, io, path::PathBuf};

use jiff::Timestamp;
use topcoat::{Result, context::Cx, router::error::internal_server_error};

use super::user::User;
use crate::db::db;

/// Use the package directory so the path does not depend on where the app starts.
const UPLOADS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/uploads");

/// Maximum file size in bytes.
pub const MAX_SIZE: usize = 10 * 1024 * 1024;

/// Maximum file name length in characters.
const MAX_NAME: usize = 120;

/// Types that browsers can show without running anything. Other files are served
/// as plain downloads, whatever type the uploader claimed.
const SAFE_TYPES: [&str; 7] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "application/pdf",
    "text/plain",
    "text/csv",
];

#[derive(Debug, toasty::Model)]
pub struct Attachment {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub issue_id: u64,

    #[index]
    pub uploader_id: u64,

    #[belongs_to]
    pub uploader: toasty::Deferred<User>,

    pub file_name: String,

    /// The media type the browser sent with the upload. Untrusted.
    content_type: String,

    /// Size in bytes.
    pub size: u64,

    pub created_at: Timestamp,
}

impl Attachment {
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Attachment>> {
        let attachment = Attachment::filter_by_id(id)
            .first()
            .exec(&mut db(cx))
            .await?;
        Ok(attachment)
    }

    /// Loads an issue's attachments and their uploaders, oldest first.
    pub async fn of_issue(cx: &Cx, issue_id: u64) -> Result<Vec<Attachment>> {
        let attachments = Attachment::filter_by_issue_id(issue_id)
            .include(Attachment::fields().uploader())
            .order_by(Attachment::fields().id().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(attachments)
    }

    /// Saves an uploaded file and attaches it to an issue. An empty or oversized
    /// file returns an error without saving anything.
    pub async fn store(
        cx: &Cx,
        issue_id: u64,
        uploader: &User,
        file_name: &str,
        content_type: &str,
        contents: &[u8],
    ) -> Result<Result<Attachment, AttachmentError>> {
        if contents.is_empty() {
            return Ok(Err(AttachmentError::Missing));
        }
        if contents.len() > MAX_SIZE {
            return Ok(Err(AttachmentError::TooLarge));
        }

        let mut db = db(cx);
        let attachment = toasty::create!(Attachment {
            issue_id,
            uploader_id: uploader.id,
            file_name: tidy_name(file_name),
            content_type: content_type.trim().to_ascii_lowercase(),
            size: contents.len() as u64,
            created_at: Timestamp::now(),
        })
        .exec(&mut db)
        .await?;

        let saved = async {
            tokio::fs::create_dir_all(UPLOADS_DIR).await?;
            tokio::fs::write(attachment.path(), contents).await
        };
        if let Err(error) = saved.await {
            // Do not keep a record without its file.
            Attachment::filter_by_id(attachment.id)
                .delete()
                .exec(&mut db)
                .await?;
            return Err(internal_server_error(error).into());
        }
        Ok(Ok(attachment))
    }

    /// Reads the file's contents.
    pub async fn contents(&self) -> Result<Vec<u8>> {
        let contents = tokio::fs::read(self.path())
            .await
            .map_err(internal_server_error)?;
        Ok(contents)
    }

    /// The media type to serve the file with.
    pub fn served_type(&self) -> &str {
        SAFE_TYPES
            .into_iter()
            .find(|safe| *safe == self.content_type)
            .unwrap_or("application/octet-stream")
    }

    /// A `Content-Disposition` header value that makes browsers save the file under
    /// its name instead of showing it.
    pub fn disposition(&self) -> String {
        let mut value = String::from("attachment; filename*=UTF-8''");
        for byte in self.file_name.bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                value.push(char::from(byte));
            } else {
                // Writing to a `String` cannot fail.
                let _ = write!(value, "%{byte:02X}");
            }
        }
        value
    }

    /// Deletes the attachment and its file.
    pub async fn remove(self, cx: &Cx) -> Result<()> {
        Attachment::filter_by_id(self.id)
            .delete()
            .exec(&mut db(cx))
            .await?;
        match tokio::fs::remove_file(self.path()).await {
            Ok(()) => Ok(()),
            // The record is gone, so a file that was already missing is fine.
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(internal_server_error(error).into()),
        }
    }

    /// Deletes every attachment of an issue.
    pub async fn remove_all(cx: &Cx, issue_id: u64) -> Result<()> {
        let attachments = Attachment::filter_by_issue_id(issue_id)
            .exec(&mut db(cx))
            .await?;
        for attachment in attachments {
            attachment.remove(cx).await?;
        }
        Ok(())
    }

    /// Where the contents are stored. The name comes from the ID, never from the
    /// uploaded file name.
    fn path(&self) -> PathBuf {
        PathBuf::from(UPLOADS_DIR).join(self.id.to_string())
    }
}

/// Keeps the last path segment of an uploaded file name and limits its length.
fn tidy_name(file_name: &str) -> String {
    let name = file_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    let name = name
        .chars()
        .filter(|char| !char.is_control())
        .take(MAX_NAME)
        .collect::<String>();
    if name.is_empty() {
        "file".to_owned()
    } else {
        name
    }
}

/// Why an upload was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentError {
    /// No file was chosen, or the file is empty.
    Missing,
    /// The file is larger than [`MAX_SIZE`].
    TooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attachment(file_name: &str, content_type: &str) -> Attachment {
        Attachment {
            id: 7,
            issue_id: 1,
            uploader_id: 1,
            uploader: toasty::Deferred::default(),
            file_name: file_name.to_owned(),
            content_type: content_type.to_owned(),
            size: 1,
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn keeps_only_the_file_name() {
        assert_eq!(tidy_name("C:\\Users\\ada\\notes.txt"), "notes.txt");
        assert_eq!(tidy_name("../../etc/passwd"), "passwd");
        assert_eq!(tidy_name("  report.pdf "), "report.pdf");
        assert_eq!(tidy_name("uploads/"), "file");
        assert_eq!(tidy_name("a\r\nb.txt"), "ab.txt");
        assert_eq!(tidy_name(&"a".repeat(500)).len(), MAX_NAME);
    }

    #[test]
    fn stores_files_by_id() {
        let path = attachment("../../evil.sh", "text/plain").path();
        assert_eq!(path, PathBuf::from(UPLOADS_DIR).join("7"));
    }

    #[test]
    fn serves_unknown_types_as_downloads() {
        assert_eq!(attachment("a.png", "image/png").served_type(), "image/png");
        assert_eq!(
            attachment("a.html", "text/html").served_type(),
            "application/octet-stream"
        );
        assert_eq!(
            attachment("a.svg", "image/svg+xml").served_type(),
            "application/octet-stream"
        );
    }

    #[test]
    fn encodes_the_download_name() {
        assert_eq!(
            attachment("Q3 plan \"v2\".pdf", "application/pdf").disposition(),
            "attachment; filename*=UTF-8''Q3%20plan%20%22v2%22.pdf"
        );
        assert_eq!(
            attachment("résumé.txt", "text/plain").disposition(),
            "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.txt"
        );
    }
}
