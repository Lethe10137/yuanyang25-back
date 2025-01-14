// @generated automatically by Diesel CLI.

diesel::table! {
    answer (id) {
        id -> Int4,
        puzzle -> Int4,
        level -> Int4,
        #[max_length = 64]
        sha256 -> Bpchar,
    }
}

diesel::table! {
    decipher (id) {
        id -> Int4,
        pricing_type -> Int4,
        base_price -> Int4,
        depth -> Int4,
        #[max_length = 64]
        root -> Bpchar,
    }
}

diesel::table! {
    final_meta_submission (id) {
        id -> Int4,
        submission_id -> Int4,
        team -> Int4,
        puzzle -> Int4,
        reward -> Int8,
        time -> Timestamptz,
    }
}

diesel::table! {
    oracle (id) {
        id -> Int4,
        puzzle -> Int4,
        team -> Int4,
        cost -> Int8,
        refund -> Int8,
        query -> Text,
        response -> Text,
    }
}

diesel::table! {
    puzzle (id) {
        id -> Int4,
        meta -> Bool,
        bounty -> Int4,
        #[max_length = 64]
        title -> Varchar,
        decipher -> Int4,
        depth -> Int4,
    }
}

diesel::table! {
    submission (id) {
        id -> Int4,
        team -> Int4,
        reward -> Int8,
        time -> Timestamptz,
        puzzle -> Int4,
        depth -> Int4,
        meta -> Bool,
    }
}

diesel::table! {
    team (id) {
        id -> Int4,
        is_staff -> Bool,
        token_balance -> Int8,
        confirmed -> Bool,
        max_size -> Int4,
        size -> Int4,
        #[max_length = 64]
        salt -> Varchar,
    }
}

diesel::table! {
    transaction (id) {
        id -> Int4,
        team -> Int4,
        #[max_length = 255]
        desp -> Varchar,
        amount -> Int8,
        balance -> Int8,
        allowance -> Int8,
        purchase_ref -> Nullable<Int4>,
        time -> Timestamptz,
    }
}

diesel::table! {
    unlock (id) {
        id -> Int4,
        team -> Int4,
        decipher -> Int4,
        level -> Int4,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 64]
        openid -> Varchar,
        team -> Nullable<Int4>,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 64]
        password -> Varchar,
        #[max_length = 64]
        salt -> Varchar,
        privilege -> Int4,
    }
}

diesel::table! {
    wrong_answer_cnt (id) {
        id -> Int4,
        team -> Int4,
        puzzle -> Int4,
        token_penalty_level -> Int4,
        time_penalty_level -> Int4,
        time_penalty_until -> Timestamptz,
    }
}

diesel::joinable!(answer -> puzzle (puzzle));
diesel::joinable!(final_meta_submission -> submission (submission_id));
diesel::joinable!(oracle -> puzzle (puzzle));
diesel::joinable!(oracle -> team (team));
diesel::joinable!(submission -> puzzle (puzzle));
diesel::joinable!(submission -> team (team));
diesel::joinable!(transaction -> team (team));
diesel::joinable!(unlock -> decipher (decipher));
diesel::joinable!(unlock -> team (team));
diesel::joinable!(wrong_answer_cnt -> puzzle (puzzle));
diesel::joinable!(wrong_answer_cnt -> team (team));

diesel::allow_tables_to_appear_in_same_query!(
    answer,
    decipher,
    final_meta_submission,
    oracle,
    puzzle,
    submission,
    team,
    transaction,
    unlock,
    users,
    wrong_answer_cnt,
);
