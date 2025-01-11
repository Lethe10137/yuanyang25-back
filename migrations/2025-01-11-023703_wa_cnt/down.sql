-- This file should undo anything in `up.sql`

-- Drop foreign key constraints for "wrong_answer_cnt"
ALTER TABLE "wrong_answer_cnt" DROP CONSTRAINT unique_team_wa;
ALTER TABLE "wrong_answer_cnt" DROP CONSTRAINT "wrong_answer_cnt_puzzle_fkey";
ALTER TABLE "wrong_answer_cnt" DROP CONSTRAINT "wrong_answer_cnt_team_fkey";

-- Drop the index on "wrong_answer_cnt"
DROP INDEX IF EXISTS "wrong_answer_cnt_index";

-- Drop the "wrong_answer_cnt" table
DROP TABLE IF EXISTS "wrong_answer_cnt";

-- Drop foreign key constraints for "mid_answer_submission"
ALTER TABLE "mid_answer_submission" DROP CONSTRAINT unique_team_midanswer;
ALTER TABLE "mid_answer_submission" DROP CONSTRAINT "mid_answer_submission_mid_answer_fkey";
ALTER TABLE "mid_answer_submission" DROP CONSTRAINT "mid_answer_submission_team_fkey";

-- Drop the index on "mid_answer_submission"
DROP INDEX IF EXISTS "mid_answer_submission_index";

-- Drop the "mid_answer_submission" table
DROP TABLE IF EXISTS "mid_answer_submission";