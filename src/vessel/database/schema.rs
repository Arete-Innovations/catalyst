// @generated automatically by Diesel CLI.

diesel::table! {
    vessels (id) {
        id -> Int4,
        name -> Text,
        display_name -> Text,
        username -> Text,
        email -> Text,
        password_hash -> Text,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        first_name -> Text,
        last_name -> Text,
    }
}
