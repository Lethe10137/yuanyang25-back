-- Your SQL goes here

CREATE TABLE "other_answer" (
    "id" SERIAL PRIMARY KEY,
    "puzzle" INTEGER NOT NULL,
    "sha256" CHAR(64) NOT NULL,
    "content" TEXT NOT NULL,
    "ref" INTEGER NOT NULL,
    CONSTRAINT "fk_puzzle_other_answer"
        FOREIGN KEY ("puzzle") REFERENCES "puzzle" ("id")
        ON DELETE CASCADE
);

CREATE INDEX "other_answer_index_0"
ON "other_answer" ("puzzle");

CREATE TABLE "other_answer_submission" (
    "id" SERIAL PRIMARY KEY,
    "team" INTEGER NOT NULL,
    "other_answer" INTEGER NOT NULL,
    "time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "other_answer_submission_unique" UNIQUE ("team", "other_answer"),
    CONSTRAINT "fk_team_other_answer_submission"
        FOREIGN KEY ("team") REFERENCES "team" ("id")
        ON DELETE CASCADE,
    CONSTRAINT "fk_other_answer_other_answer_submission"
        FOREIGN KEY ("other_answer") REFERENCES "other_answer" ("id")
        ON DELETE CASCADE
);

-- 为 team 创建索引以优化基于 team 的查询
CREATE INDEX "other_answer_submission_team_index"
ON "other_answer_submission" ("team");