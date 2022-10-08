pub mod resource {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize)]
    pub struct CreateUserDto<'a> {
        pub username: &'a str,
        pub email: &'a str,
        pub password: &'a str,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct UpdateUserDto<'a> {
        email: Option<&'a str>,
        password: Option<&'a str>,
        username: Option<&'a str>,
        bio: Option<&'a str>,
        image: Option<&'a str>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AuthenticateUserDto<'a> {
        email: &'a str,
        password: &'a str,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ProfileDto<'a> {
        username: &'a str,
        bio: &'a str,
        image: &'a str,
        following: bool,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CreateArticleDto<'a> {
        title: &'a str,
        description: &'a str,
        body: &'a str,
        #[serde(rename = "tagList")]
        tag_list: Vec<&'a str>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct UpdateArticleDto<'a> {
        title: &'a str,
        description: &'a str,
        body: &'a str,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CreateArticleCommentDto<'a> {
        body: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct UserResponse {
        pub email: String,
        pub password: String,
        pub username: String,
        pub bio: String,
        pub image: String,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ProfileResponse {
        pub username: String,
        pub bio: String,
        pub image: String,
        pub following: bool,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleCommentResponse {
        pub id: u64,
        pub body: String,
        pub author: ProfileResponse,
        #[serde(rename = "createdAt")]
        pub created: DateTime<Utc>,
        #[serde(rename = "updatedAt")]
        pub updated: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleResponse {
        pub slug: String,
        pub title: String,
        pub description: String,
        pub body: String,
        pub author: ProfileResponse,
        #[serde(rename = "tagList")]
        pub tag_list: Vec<String>,
        pub favorited: bool,
        #[serde(rename = "favoritesCount")]
        pub favorites_count: u32,
        #[serde(rename = "createdAt")]
        pub created: DateTime<Utc>,
        #[serde(rename = "updatedAt")]
        pub updated: DateTime<Utc>,
    }
}

pub mod transform {
    pub mod user {
        use crate::{
            app::resource::{CreateUserDto, UserResponse},
            domain::entity::User,
        };

        impl<'a> From<CreateUserDto<'a>> for User {
            fn from(dto: CreateUserDto) -> Self {
                Self {
                    email: dto.email.into(),
                    bio: "".into(),
                    image: "".into(),
                    password: dto.password.into(),
                    username: dto.username.into(),
                }
            }
        }

        impl From<User> for UserResponse {
            fn from(user: User) -> Self {
                Self {
                    email: user.email,
                    bio: user.bio,
                    image: user.image,
                    password: user.password,
                    username: user.username,
                }
            }
        }
    }
}
