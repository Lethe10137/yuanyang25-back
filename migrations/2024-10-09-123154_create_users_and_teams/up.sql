CREATE TABLE "users" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"openid" VARCHAR(64) NOT NULL UNIQUE,
	"team" INTEGER,
	"username" VARCHAR(255) NOT NULL,
	"password" VARCHAR(64) NOT NULL,
	"salt" VARCHAR(64) NOT NULL,
	"privilege" INTEGER NOT NULL DEFAULT 0,
	PRIMARY KEY("id")
);

CREATE INDEX "users_index_0"
ON "users" ("id");

CREATE INDEX "users_index_1"
ON "users" ("openid");

CREATE TABLE "team" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"is_staff" BOOLEAN NOT NULL DEFAULT false,
	"token_balance" BIGINT NOT NULL DEFAULT 0,
	"confirmed" BOOLEAN NOT NULL DEFAULT false,
	"max_size" INTEGER NOT NULL DEFAULT 3,
	"size" INTEGER NOT NULL DEFAULT 0,
	"salt" VARCHAR(64) NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "group_index_0"
ON "team" ("id");

CREATE TABLE "puzzle" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"bounty" INTEGER NOT NULL,
	"title" VARCHAR(64) NOT NULL,
	"answer" VARCHAR(64) NOT NULL,
	"key" VARCHAR(64) NOT NULL,
	"content" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "puzzle_index_0"
ON "puzzle" ("id");

CREATE TABLE "submission" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"team" INTEGER NOT NULL,
	"reward" BIGINT NOT NULL,
	"time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
	"puzzle" INTEGER NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "submission_index_0"
ON "submission" ("team");

CREATE INDEX "submission_index_1"
ON "submission" ("puzzle");

CREATE INDEX "submission_index_2"
ON "submission" ("puzzle", "team");

CREATE TABLE "mid_answer" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"puzzle" INTEGER NOT NULL,
	"query" VARCHAR(64) NOT NULL,
	"response" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "mid_answer_index_0"
ON "mid_answer" ("puzzle");

CREATE TABLE "hint" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"title" VARCHAR(64) NOT NULL,
	"base_price" BIGINT NOT NULL,
	"puzzle" INTEGER NOT NULL,
	"content" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "hint_index_0"
ON "hint" ("puzzle");

CREATE TABLE "oracle" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"puzzle" INTEGER NOT NULL,
	"team" INTEGER NOT NULL,
	"cost" BIGINT NOT NULL,
	"refund" BIGINT NOT NULL DEFAULT 0,
	"query" TEXT NOT NULL,
	"response" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "oracle_index_0"
ON "oracle" ("puzzle", "team");

CREATE TABLE "unlock" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
	"team" INTEGER NOT NULL,
	"puzzle" INTEGER NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "unlock_index_0"
ON "unlock" ("team");

CREATE INDEX "unlock_team_puzzle_idx"
ON "unlock" ("team", "puzzle");

CREATE TABLE "transaction" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"team" INTEGER NOT NULL,
	"desp" VARCHAR(255) NOT NULL,
	"amount" BIGINT NOT NULL,
	"balance" BIGINT NOT NULL,
	"time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY("id")
);

CREATE INDEX "transaction_index_0"
ON "transaction" ("team");

ALTER TABLE "users"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "submission"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "submission"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "mid_answer"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "hint"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "oracle"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "oracle"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "unlock"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "unlock"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "transaction"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
