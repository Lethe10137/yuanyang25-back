-- This file should undo anything in `up.sql`
-- 删除触发器
DROP TRIGGER IF EXISTS update_vericode_updated_at ON vericode;

-- 删除触发器函数
DROP FUNCTION IF EXISTS update_updated_at_column;

-- 删除 vericode 表
DROP TABLE IF EXISTS vericode;