CREATE TABLE "projects" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "code" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "lead_id" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_projects_by_code" ON "projects" ("code");
-- #[toasty::breakpoint]
CREATE INDEX "index_projects_by_lead_id" ON "projects" ("lead_id");
-- #[toasty::breakpoint]
CREATE TABLE "issues" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "project_id" INTEGER NOT NULL,
    "number" INTEGER NOT NULL,
    "title" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('task', 'bug', 'feature')),
    "status" TEXT NOT NULL CHECK ("status" IN ('backlog', 'todo', 'in_progress', 'in_review', 'done')),
    "priority" TEXT NOT NULL CHECK ("priority" IN ('1_low', '2_medium', '3_high', '4_urgent')),
    "assignee_id" INTEGER,
    "reporter_id" INTEGER NOT NULL,
    "due_on" TEXT,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    "completed_at" TEXT
);
-- #[toasty::breakpoint]
CREATE INDEX "index_issues_by_status" ON "issues" ("status");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_issues_by_project_id_and_number" ON "issues" ("project_id", "number");
-- #[toasty::breakpoint]
CREATE INDEX "index_issues_by_project_id" ON "issues" ("project_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_issues_by_assignee_id" ON "issues" ("assignee_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_issues_by_reporter_id" ON "issues" ("reporter_id");
-- #[toasty::breakpoint]
CREATE TABLE "users" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "password_hash" TEXT NOT NULL,
    "role" TEXT NOT NULL CHECK ("role" IN ('admin', 'staff', 'viewer'))
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_users_by_email" ON "users" ("email");
-- #[toasty::breakpoint]
CREATE TABLE "sessions" (
    "token_hash" BLOB NOT NULL,
    "user_id" INTEGER NOT NULL,
    "expires_at" TEXT NOT NULL,
    PRIMARY KEY ("token_hash")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_sessions_by_user_id" ON "sessions" ("user_id");
-- #[toasty::breakpoint]
CREATE TABLE "comments" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "issue_id" INTEGER NOT NULL,
    "author_id" INTEGER NOT NULL,
    "body" TEXT NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_comments_by_issue_id" ON "comments" ("issue_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_comments_by_author_id" ON "comments" ("author_id");
