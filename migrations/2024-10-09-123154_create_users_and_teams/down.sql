

-- 先删除外键约束
ALTER TABLE "users" DROP CONSTRAINT "users_team_fkey";
ALTER TABLE "submission" DROP CONSTRAINT "submission_team_fkey";
ALTER TABLE "submission" DROP CONSTRAINT "submission_puzzle_fkey";
ALTER TABLE "mid_answer" DROP CONSTRAINT "mid_answer_puzzle_fkey";
ALTER TABLE "hint" DROP CONSTRAINT "hint_puzzle_fkey";
ALTER TABLE "oracle" DROP CONSTRAINT "oracle_puzzle_fkey";
ALTER TABLE "oracle" DROP CONSTRAINT "oracle_team_fkey";
ALTER TABLE "unlock" DROP CONSTRAINT "unlock_puzzle_fkey";
ALTER TABLE "unlock" DROP CONSTRAINT "unlock_team_fkey";
ALTER TABLE "transaction" DROP CONSTRAINT "transaction_team_fkey";

-- 删除索引
DROP INDEX IF EXISTS "users_index_0";
DROP INDEX IF EXISTS "users_index_1";
DROP INDEX IF EXISTS "group_index_0";
DROP INDEX IF EXISTS "puzzle_index_0";
DROP INDEX IF EXISTS "submission_index_0";
DROP INDEX IF EXISTS "submission_index_1";
DROP INDEX IF EXISTS "submission_index_2";
DROP INDEX IF EXISTS "mid_answer_index_0";
DROP INDEX IF EXISTS "hint_index_0";
DROP INDEX IF EXISTS "oracle_index_0";
DROP INDEX IF EXISTS "unlock_index_0";
DROP INDEX IF EXISTS "transaction_index_0";

-- 删除表
DROP TABLE IF EXISTS "users";
DROP TABLE IF EXISTS "team";
DROP TABLE IF EXISTS "puzzle";
DROP TABLE IF EXISTS "submission";
DROP TABLE IF EXISTS "mid_answer";
DROP TABLE IF EXISTS "hint";
DROP TABLE IF EXISTS "oracle";
DROP TABLE IF EXISTS "unlock";
DROP TABLE IF EXISTS "transaction";
