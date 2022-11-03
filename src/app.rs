pub mod resource {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    pub mod iam {
        use super::*;

        #[derive(Debug, Clone, Deserialize)]
        pub struct CreateUserDto<'a> {
            pub username: &'a str,
            pub email: &'a str,
            pub password: &'a str,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct UpdateUserDto<'a> {
            bio: &'a str,
            image_url: &'a str,
        }

        #[derive(Debug, Clone, Serialize)]
        pub struct UserResponse {
            pub id: Uuid,
            pub username: String,
            pub email: String,
            pub bio: Option<String>,
            pub image_url: Option<String>,
            pub created: DateTime<Utc>,
            pub updated: Option<DateTime<Utc>>,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct AuthenticateUserDto<'a> {
            email: &'a str,
            password: &'a str,
        }
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ProfileResponse {
        pub id: Uuid,
        pub username: String,
        pub bio: String,
        pub image_url: String,
        pub created: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutFollowDto<'a> {
        pub following_id: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct FollowResponse {
        pub id: Uuid,
        pub follower_id: Uuid,
        pub following_id: Uuid,
        pub created: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleDto<'a> {
        title: &'a str,
        description: &'a str,
        body: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleResponse {
        pub id: Uuid,
        pub slug: String,
        pub title: String,
        pub description: String,
        pub body: String,
        pub tags: Vec<String>,
        pub version: u32,
        pub version_id: Uuid,
        pub author_id: Uuid,
        pub created: DateTime<Utc>,
        pub updated: Option<DateTime<Utc>>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleFavorite<'a> {
        pub article_id: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleFavoriteResponse {
        pub id: Uuid,
        pub article_id: Uuid,
        pub profile_id: Uuid,
        pub created: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleComment<'a> {
        pub article_id: &'a str,
        pub message: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleCommentResponse {
        pub id: Uuid,
        pub article_id: Uuid,
        pub profile_id: Uuid,
        pub message: String,
        pub edited: bool,
        pub created: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleCommentVote<'a> {
        pub comment_id: &'a str,
        pub reaction: &'a str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ArticleCOmmentVoteResponse {
        pub id: Uuid,
        pub article_id: Uuid,
        pub profile_id: Uuid,
        pub comment_id: Uuid,
        pub reaction: String,
        pub created: DateTime<Utc>,
    }
}

pub mod transform {
    pub mod user {
        use crate::{
            app::resource::iam::{CreateUserDto, UserResponse},
            domain::entity::{Entity, User, UserState},
        };

        impl<'a> From<CreateUserDto<'a>> for User {
            fn from(dto: CreateUserDto) -> Self {
                Self::new(UserState::from(dto))
            }
        }

        impl From<User> for UserResponse {
            fn from(user: User) -> Self {
                Self {
                    id: user.ident(),
                    username: user.username(),
                    email: user.email(),
                    bio: user.bio(),
                    image_url: user.image_url(),
                    created: user.created(),
                    updated: user.updated(),
                }
            }
        }
    }
}
