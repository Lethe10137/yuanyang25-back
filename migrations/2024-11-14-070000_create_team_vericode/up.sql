-- Your SQL goes here

-- 创建 vericode 表
CREATE TABLE vericode (
    id SERIAL PRIMARY KEY,
    code VARCHAR(16) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    team_id INT UNIQUE NOT NULL REFERENCES team(id) ON DELETE CASCADE
);

-- 创建触发器，自动更新 updated_at 字段
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_vericode_updated_at
BEFORE UPDATE ON vericode
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();