
-- 增加 active 字段
ALTER TABLE "oracle"
ADD COLUMN "active" BOOLEAN NOT NULL DEFAULT FALSE;

-- 创建复合索引，用于查询 (puzzle, team, active)
CREATE INDEX "oracle_index_active"
ON "oracle" ("puzzle", "team", "active");

-- 创建索引，用于更新操作 (id, team)
CREATE INDEX "oracle_index_update"
ON "oracle" ("id", "team");

CREATE INDEX "oracle_index_id_active"
ON "oracle" ("id", "active");