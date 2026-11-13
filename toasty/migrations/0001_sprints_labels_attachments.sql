ALTER TABLE "issues" ADD COLUMN "sprint_id" INTEGER;
-- #[toasty::breakpoint]
CREATE INDEX "index_issues_by_sprint_id" ON "issues" ("sprint_id");
-- #[toasty::breakpoint]
-- Edited by hand: the default makes existing members active. Without it, SQLite
-- refuses to add a required column to a table that has rows.
ALTER TABLE "users" ADD COLUMN "active" BOOLEAN NOT NULL DEFAULT 1;
-- #[toasty::breakpoint]
CREATE TABLE "attachments" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "issue_id" INTEGER NOT NULL,
    "uploader_id" INTEGER NOT NULL,
    "file_name" TEXT NOT NULL,
    "content_type" TEXT NOT NULL,
    "size" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_attachments_by_issue_id" ON "attachments" ("issue_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_attachments_by_uploader_id" ON "attachments" ("uploader_id");
-- #[toasty::breakpoint]
CREATE TABLE "issue_labels" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "issue_id" INTEGER NOT NULL,
    "label_id" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_issue_labels_by_issue_id_and_label_id" ON "issue_labels" ("issue_id", "label_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_issue_labels_by_issue_id" ON "issue_labels" ("issue_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_issue_labels_by_label_id" ON "issue_labels" ("label_id");
-- #[toasty::breakpoint]
CREATE TABLE "sprints" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "project_id" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "goal" TEXT NOT NULL,
    "starts_on" TEXT NOT NULL,
    "ends_on" TEXT NOT NULL,
    "state" TEXT NOT NULL CHECK ("state" IN ('planned', 'active', 'completed'))
);
-- #[toasty::breakpoint]
CREATE INDEX "index_sprints_by_project_id" ON "sprints" ("project_id");
-- #[toasty::breakpoint]
CREATE TABLE "labels" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "color" TEXT NOT NULL CHECK ("color" IN ('gray', 'red', 'amber', 'green', 'sky', 'violet', 'pink'))
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_labels_by_name" ON "labels" ("name");
