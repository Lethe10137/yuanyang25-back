-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS "other_answer_submission_team_index";
DROP TABLE IF EXISTS "other_answer_submission";

DROP INDEX IF EXISTS "other_answer_index_0";
DROP TABLE IF EXISTS "other_answer";
