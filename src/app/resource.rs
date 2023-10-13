macro_rules! resource_response {
    (struct $name:ident; $($field:ident: $field_ty:ty),+ ,) => {
		#[derive(core::fmt::Debug, core::clone::Clone, serde::Serialize)]
        pub struct $name {
            pub id: Uuid,
            pub created: DateTime<Utc>,
            pub updated: Option<DateTime<Utc>>,
            pub version: u32,
            $(pub $field: $field_ty),+
        }
    };
}

pub mod iam {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use url::Url;
    use uuid::Uuid;

    use crate::base::resource_id;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateUser<'a> {
        pub username: &'a str,
        pub email: &'a str,
        pub password: &'a str,
    }

    resource_id!(CreateUser<'_>, "iam::CreateUser");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UpdateUser {
        pub bio: Option<String>,
        pub image_url: Option<Url>,
    }

    resource_id!(UpdateUser, "iam::UpdateUser");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserCredential<'a> {
        pub email: &'a str,
        pub password: &'a str,
    }

    resource_id!(UserCredential<'_>, "iam::UserCredential");

    resource_response! {
        struct UserResponse;
        username: String,
        email: String,
        bio: Option<String>,
        image_url: Option<Url>,
    }

    resource_id!(UserResponse, "iam::User");

    #[derive(Debug, Clone, Serialize)]
    pub struct AuthenticateUserResponse {
        pub user: UserResponse,
        pub token: String,
    }

    resource_id!(AuthenticateUserResponse, "iam::AuthenticateUser");
}

pub mod profile {
    use chrono::{DateTime, Utc};
    use serde::Deserialize;
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutFollowDto<'a> {
        pub following_id: &'a str,
    }

    resource_response! {
        struct ProfileResponse;
        username: String,
        bio: String,
        image_url: String,
    }

    resource_response! {
        struct FollowResponse;
        follower_id: Uuid,
        following_id: Uuid,
    }
}

pub mod article {
    use chrono::{DateTime, Utc};
    use serde::Deserialize;
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleDto<'a> {
        title: &'a str,
        description: &'a str,
        body: &'a str,
    }

    resource_response! {
        struct ArticleResponse;
        slug: String,
        title: String,
        description: String,
        body: String,
        tags: Vec<String>,
        author_id: Uuid,
        version_id: Uuid,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleFavorite<'a> {
        pub article_id: &'a str,
    }

    resource_response! {
        struct ArticleFavoriteResponse;
        article_id: Uuid,
        profile_id: Uuid,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleComment<'a> {
        pub article_id: &'a str,
        pub message: &'a str,
    }

    resource_response! {
        struct ArticleCommentResponse;
        article_id: Uuid,
        profile_id: Uuid,
        message: String,
        edited: bool,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PutArticleCommentVote<'a> {
        pub comment_id: &'a str,
        pub reaction: &'a str,
    }

    resource_response! {
        struct ArticleCommentVoteResponse;
        profile_id: Uuid,
        article_id: Uuid,
        comment_id: Uuid,
        reaction: String,
    }
}
