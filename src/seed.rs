use IssueKind::{Bug, Feature, Task};
use jiff::{SignedDuration, Timestamp};
use toasty::Db;

use crate::features::{
    issue::{Comment, Issue, IssueKind, IssueStatus, Priority},
    project::Project,
    stats::today,
    user::{Role, User, hash_password},
};

pub const DEMO_PASSWORD: &str = "trackline";

/// The accounts listed on the sign-in page, one per role.
pub const DEMO_USERS: [(&str, &str, Role); 3] = [
    ("Ada Lovelace", "ada@trackline.test", Role::Admin),
    ("Grace Hopper", "grace@trackline.test", Role::Staff),
    ("Alan Turing", "alan@trackline.test", Role::Viewer),
];

/// More members so issues have a team to be assigned to.
const TEAMMATES: [(&str, &str); 5] = [
    ("Katherine Johnson", "katherine@trackline.test"),
    ("Edsger Dijkstra", "edsger@trackline.test"),
    ("Hedy Lamarr", "hedy@trackline.test"),
    ("Dennis Ritchie", "dennis@trackline.test"),
    ("Annie Easley", "annie@trackline.test"),
];

struct ProjectSeed {
    name: &'static str,
    code: &'static str,
    description: &'static str,
    issues: &'static [(IssueKind, &'static str)],
}

const PROJECTS: [ProjectSeed; 5] = [
    ProjectSeed {
        name: "Web App",
        code: "WEB",
        description: "The customer-facing web application and its checkout.",
        issues: &[
            (Feature, "Add saved payment methods to checkout"),
            (Bug, "Cart badge shows a stale count after removing an item"),
            (Task, "Upgrade the router to the latest major version"),
            (Bug, "Password reset link expires too early"),
            (Feature, "Let customers reorder a past purchase"),
            (Bug, "Search results jump when images load"),
            (Task, "Remove the legacy promo banner code"),
            (Feature, "Show estimated delivery dates on product pages"),
            (Bug, "Address form rejects postcodes with spaces"),
            (Task, "Add end-to-end tests for guest checkout"),
            (Feature, "Dark mode for the account area"),
            (Bug, "Double submit creates duplicate orders"),
            (Task, "Audit pages for keyboard accessibility"),
            (Feature, "Wishlist sharing by link"),
            (Bug, "Session ends while a payment is in progress"),
            (Task, "Move feature flags to the new config service"),
            (Bug, "Discount codes ignore letter case"),
            (Feature, "Product comparison view"),
            (Task, "Reduce the main bundle below 200 kB"),
            (Bug, "Order history pagination skips a page"),
            (Feature, "Inline validation for the sign-up form"),
            (Task, "Document the release checklist"),
        ],
    },
    ProjectSeed {
        name: "Mobile App",
        code: "MOB",
        description: "The iOS and Android apps.",
        issues: &[
            (Feature, "Biometric sign-in"),
            (Bug, "App freezes when the network drops during sync"),
            (Task, "Update store screenshots for the new design"),
            (Bug, "Push notifications open the wrong screen"),
            (Feature, "Offline mode for the order list"),
            (Bug, "Keyboard covers the comment field on small phones"),
            (Task, "Raise the minimum supported Android version"),
            (Feature, "Home screen widget for open orders"),
            (Bug, "Images are blurry on tablets"),
            (Task, "Set up crash reporting alerts"),
            (Feature, "Scan a barcode to find a product"),
            (Bug, "Back gesture closes the app from the cart"),
            (Task, "Profile cold start time"),
            (Feature, "Deep links for product pages"),
            (Bug, "Date picker ignores the device language"),
            (Task, "Remove the unused analytics SDK"),
            (Bug, "Pull to refresh shows the spinner forever"),
            (Feature, "Haptic feedback on key actions"),
            (Task, "Automate beta builds from the main branch"),
            (Bug, "Text is cut off with large font sizes"),
            (Feature, "Share a product from the detail screen"),
            (Task, "Write the release notes for version 4.2"),
        ],
    },
    ProjectSeed {
        name: "API Platform",
        code: "API",
        description: "Public and internal APIs, authentication, and webhooks.",
        issues: &[
            (Feature, "Cursor pagination for list endpoints"),
            (Bug, "Webhook retries stop after the first failure"),
            (Task, "Publish the OpenAPI document with each release"),
            (Bug, "Rate limit headers report the wrong reset time"),
            (Feature, "Scoped API keys"),
            (Bug, "Bulk import times out above 5,000 rows"),
            (Task, "Deprecate version 1 of the orders endpoint"),
            (Feature, "Idempotency keys for payment requests"),
            (Bug, "Invalid JSON returns a 500 instead of a 400"),
            (Task, "Add tracing to the checkout service"),
            (Feature, "Webhook signature verification helpers"),
            (Bug, "Filtering by date ignores the time zone"),
            (Task, "Load test the search endpoint"),
            (Feature, "Sandbox environment for partners"),
            (Bug, "Tokens stay valid after a password change"),
            (Task, "Rotate the signing keys"),
            (Bug, "Sorting by name is case sensitive"),
            (Feature, "Batch endpoint for stock updates"),
            (Task, "Write a migration guide for version 2"),
            (Bug, "Empty arrays are serialized as null"),
            (Feature, "Audit log for admin actions"),
            (Task, "Cache product lookups"),
        ],
    },
    ProjectSeed {
        name: "Design System",
        code: "DSN",
        description: "Shared components, tokens, and guidelines.",
        issues: &[
            (Feature, "Date range picker component"),
            (Bug, "Focus ring is invisible on dark backgrounds"),
            (Task, "Document spacing tokens"),
            (Bug, "Dialog does not trap focus"),
            (Feature, "Empty state illustrations"),
            (Bug, "Tooltip overflows the viewport on the right edge"),
            (Task, "Audit color contrast for every badge"),
            (Feature, "Data table with sticky headers"),
            (Bug, "Select arrow overlaps long labels"),
            (Task, "Publish the icon set as a package"),
            (Feature, "Skeleton loading components"),
            (Bug, "Button height differs between browsers"),
            (Task, "Write guidelines for form errors"),
            (Feature, "Toast notifications"),
            (Bug, "Avatar initials are not centered"),
            (Task, "Remove deprecated color names"),
            (Feature, "Compact density for tables"),
            (Bug, "Tabs lose their state on resize"),
            (Task, "Add visual regression tests"),
            (Feature, "Stepper for multi-page forms"),
            (Bug, "Checkbox label is not clickable"),
            (Task, "Review right-to-left layouts"),
        ],
    },
    ProjectSeed {
        name: "Infrastructure",
        code: "OPS",
        description: "Deployments, monitoring, and developer tooling.",
        issues: &[
            (Task, "Upgrade the database to the next major version"),
            (Bug, "Nightly backups fail without an alert"),
            (Feature, "Preview environments for pull requests"),
            (Bug, "Deploys hang when a health check is slow"),
            (Task, "Move secrets to the managed vault"),
            (Feature, "Dashboard for build times"),
            (Bug, "Log search drops lines over 16 kB"),
            (Task, "Set memory limits for every service"),
            (Feature, "One-command local setup"),
            (Bug, "Staging certificates expire unnoticed"),
            (Task, "Cut the test suite time in half"),
            (Feature, "Automatic rollbacks on error spikes"),
            (Bug, "Cron jobs run twice after a failover"),
            (Task, "Write the incident response runbook"),
            (Feature, "Cost report per team"),
            (Bug, "CDN serves stale assets after a release"),
            (Task, "Archive unused container images"),
            (Feature, "Canary releases for the API"),
            (Bug, "Alerts page the wrong on-call person"),
            (Task, "Test restoring from a backup"),
            (Feature, "Self-service database copies"),
            (Task, "Document the on-call handover"),
        ],
    },
];

const COMMENTS: [&str; 12] = [
    "I can reproduce this on the latest build.",
    "Picking this up today.",
    "Opened a pull request. It needs one more review.",
    "Blocked until the API change is released.",
    "This is bigger than it looks. I split the rest into a follow-up.",
    "Looks good to me.",
    "Can we get this in before the next release?",
    "Design is ready and linked in the description.",
    "Tested on staging and it works as described.",
    "Moving this back. The fix caused a regression.",
    "Added notes from the customer call.",
    "Do we still need this after last week's changes?",
];

/// How far back the sample issues go.
const HISTORY: SignedDuration = SignedDuration::from_hours(45 * 24);

const HOUR: SignedDuration = SignedDuration::from_hours(1);

/// Adds team members if there are none. Adds the sample projects and issues if
/// there are no projects. Existing data is otherwise left alone.
pub async fn seed(db: &Db) -> toasty::Result<()> {
    let mut db = db.clone();

    if User::all().first().exec(&mut db).await?.is_none() {
        seed_users(&mut db).await?;
    }
    if Project::all().first().exec(&mut db).await?.is_none() {
        seed_workspace(&mut db).await?;
    }

    Ok(())
}

async fn seed_users(db: &mut Db) -> toasty::Result<()> {
    let teammates = TEAMMATES.map(|(name, email)| (name, email, Role::Staff));
    for (name, email, role) in DEMO_USERS.into_iter().chain(teammates) {
        let password_hash = tokio::task::spawn_blocking(|| hash_password(DEMO_PASSWORD))
            .await
            .expect("hashing a password does not panic");

        toasty::create!(User {
            name,
            email,
            password_hash: &password_hash,
            role,
        })
        .exec(db)
        .await?;
    }

    Ok(())
}

async fn seed_workspace(db: &mut Db) -> toasty::Result<()> {
    let mut rng = Rng(0x5eed);
    let now = Timestamp::now();

    let users = User::all().exec(db).await?;
    let members = users
        .iter()
        .filter(|user| user.role.can_edit())
        .collect::<Vec<_>>();
    if members.is_empty() {
        return Ok(());
    }

    let mut tx = db.transaction().await?;

    let mut projects = Vec::with_capacity(PROJECTS.len());
    for (index, seed) in PROJECTS.iter().enumerate() {
        let project = toasty::create!(Project {
            name: seed.name,
            code: seed.code,
            description: seed.description,
            lead_id: members[index % members.len()].id,
            created_at: now - HISTORY,
        })
        .exec(&mut tx)
        .await?;
        projects.push(project);
    }

    // Insert in date order so sorting by ID also sorts by date.
    let mut planned = Vec::new();
    for (project, seed) in projects.iter().zip(&PROJECTS) {
        for &(kind, title) in seed.issues {
            planned.push((now - rng.duration(HISTORY), project, kind, title));
        }
    }
    planned.sort_by_key(|&(created_at, ..)| created_at);

    let mut numbers = vec![0_u32; projects.len()];
    for (created_at, project, kind, title) in planned {
        let age = now.duration_since(created_at);
        let status = status_for_age(&mut rng, age);
        // Most work is finished within ten days.
        let completed_at = (status == IssueStatus::Done)
            .then(|| (created_at + rng.duration(age.clamp(HOUR, HOUR * 240))).min(now));
        let updated_at =
            completed_at.unwrap_or_else(|| (created_at + rng.duration(age.max(HOUR))).min(now));
        let unassigned = match status {
            IssueStatus::Backlog => rng.below(100) < 60,
            IssueStatus::Todo => rng.below(100) < 20,
            _ => false,
        };
        let due_on = (rng.below(100) < 45).then(|| {
            let days = i64::from(5 + rng.below(25));
            today(created_at + SignedDuration::from_hours(24 * days))
        });

        let position = projects
            .iter()
            .position(|other| other.id == project.id)
            .expect("the project is in the list");
        numbers[position] += 1;

        let issue = toasty::create!(Issue {
            project_id: project.id,
            number: numbers[position],
            title,
            description: description(kind),
            kind,
            status,
            priority: priority(&mut rng),
            assignee_id: (!unassigned).then(|| rng.pick(&members).id),
            reporter_id: rng.pick(&members).id,
            due_on,
            created_at,
            updated_at,
            completed_at,
        })
        .exec(&mut tx)
        .await?;

        if status == IssueStatus::Backlog {
            continue;
        }
        let mut written = (0..rng.below(4))
            .map(|_| created_at + rng.duration(updated_at.duration_since(created_at) + HOUR))
            .collect::<Vec<_>>();
        written.sort();
        for created_at in written {
            toasty::create!(Comment {
                issue_id: issue.id,
                author_id: rng.pick(&members).id,
                body: *rng.pick(&COMMENTS),
                created_at: created_at.min(now),
            })
            .exec(&mut tx)
            .await?;
        }
    }

    tx.commit().await
}

/// Older issues are mostly done. Recent ones are spread across the board.
fn status_for_age(rng: &mut Rng, age: SignedDuration) -> IssueStatus {
    // Chances in percent of each status before `Done`, in board order.
    let chances = match age.as_hours() / 24 {
        0..7 => [20, 30, 25, 15],
        7..21 => [10, 12, 14, 14],
        _ => [5, 4, 3, 3],
    };
    let mut roll = rng.below(100);
    for (status, chance) in IssueStatus::ALL.into_iter().zip(chances) {
        if roll < chance {
            return status;
        }
        roll -= chance;
    }
    IssueStatus::Done
}

fn priority(rng: &mut Rng) -> Priority {
    match rng.below(100) {
        0..10 => Priority::Urgent,
        10..35 => Priority::High,
        35..80 => Priority::Medium,
        _ => Priority::Low,
    }
}

fn description(kind: IssueKind) -> &'static str {
    match kind {
        IssueKind::Bug => {
            "Steps to reproduce:\n1. Open the affected screen.\n2. Follow the flow in the \
             title.\n\nExpected: it works as documented.\nActual: see the title.\n\n\
             Reported by support. Several customers ran into this last week."
        }
        IssueKind::Feature => {
            "Why: customers keep asking for this, and it came up in the last three \
             interviews.\n\nScope:\n- A first version behind a feature flag\n- Usage \
             tracking\n- A short help article\n\nOut of scope: anything that needs a \
             data migration."
        }
        IssueKind::Task => {
            "Housekeeping that keeps the project healthy.\n\nDone when the change is \
             merged, documented, and announced in the team channel."
        }
    }
}

/// Xorshift64 generator for sample data. Its output is predictable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Generates a value in `0..bound`.
    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.next() % u64::from(bound)).expect("the remainder is below `bound`")
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let index = self.next() % items.len() as u64;
        &items[usize::try_from(index).expect("the remainder is below the length")]
    }

    /// Generates a duration in `0..max`.
    fn duration(&mut self, max: SignedDuration) -> SignedDuration {
        let secs = self.next() % max.as_secs().unsigned_abs();
        SignedDuration::from_secs(i64::try_from(secs).expect("the remainder is below `max`"))
    }
}
