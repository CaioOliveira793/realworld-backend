pub mod entity {
    #[derive(Debug, Clone)]
    pub struct User {
        pub email: String,
        pub password: String,
        pub username: String,
        pub bio: String,
        pub image: String,
    }
}
