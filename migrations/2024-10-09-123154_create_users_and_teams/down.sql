
-- 删除索引
DROP INDEX IF EXISTS "wrong_answer_cnt_index_team_puzzle";
DROP INDEX IF EXISTS "unlock_index_team_decipher";
DROP INDEX IF EXISTS "transaction_index_0";
DROP INDEX IF EXISTS "oracle_index_0";
DROP INDEX IF EXISTS "submission_index_team_depth";
DROP INDEX IF EXISTS "submission_index_puzzle_depth";
DROP INDEX IF EXISTS "answer_index_0";
DROP INDEX IF EXISTS "users_index_1";
DROP INDEX IF EXISTS "unique_team_purchase";

-- 删除表
DROP TABLE IF EXISTS "wrong_answer_cnt" CASCADE;
DROP TABLE IF EXISTS "unlock" CASCADE;
DROP TABLE IF EXISTS "decipher" CASCADE;
DROP TABLE IF EXISTS "transaction" CASCADE;
DROP TABLE IF EXISTS "oracle" CASCADE;
DROP TABLE IF EXISTS "submission" CASCADE;
DROP TABLE IF EXISTS "answer" CASCADE;
DROP TABLE IF EXISTS "puzzle" CASCADE;
DROP TABLE IF EXISTS "team" CASCADE;
DROP TABLE IF EXISTS "users" CASCADE;
