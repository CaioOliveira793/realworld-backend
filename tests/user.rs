use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use uuid::Uuid;

use crate::setup::setup_test;

mod setup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredential<'a> {
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub version: u32,
    pub email: String,
    pub username: String,
    pub bio: Option<String>,
    pub image_url: Option<String>,
}

mod create_user {
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn create_user() {
        let (client, url, _) = setup_test().await;

        let dto = CreateUser {
            email: "some@email.com",
            username: "user12345",
            password: "secure:12345678",
        };

        let req = client
            .post(url.join("/api/user").unwrap())
            .json(&dto)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();

        assert_eq!(
            res.status(),
            StatusCode::CREATED,
            "invalid created user status code"
        );

        let user: UserResponse = res.json().await.unwrap();

        assert_eq!(user.email, dto.email);
        assert_eq!(user.username, dto.username);
        assert_eq!(user.bio, None);
        assert_eq!(user.image_url, None);
    }

    #[tokio::test]
    #[serial]
    async fn validate_duplicated_data() {
        let (client, url, _) = setup_test().await;

        let dto = CreateUser {
            email: "same@email.com",
            username: "user12345",
            password: "secure:12345678",
        };

        let req = client
            .post(url.join("/api/user").unwrap())
            .json(&dto)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let req = client
            .post(url.join("/api/user").unwrap())
            .json(&dto)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

mod authenticate_user {
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn authenticate_user() {
        let (client, url, _) = setup_test().await;

        let dto = CreateUser {
            email: "user@email.com",
            username: "user12345",
            password: "12345678",
        };

        let req = client
            .post(url.join("/api/user").unwrap())
            .json(&dto)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let credential = UserCredential {
            email: "user@email.com",
            password: "12345678",
        };

        let req = client
            .post(url.join("/api/auth").unwrap())
            .json(&credential)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn validate_inexistent_user() {
        let (client, url, _) = setup_test().await;

        let credential = UserCredential {
            email: "user@email.com",
            password: "12345678",
        };

        let req = client
            .post(url.join("/api/auth").unwrap())
            .json(&credential)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn verify_user_password() {
        let (client, url, _) = setup_test().await;

        let dto = CreateUser {
            email: "user@email.com",
            username: "user12345",
            password: "12345678",
        };

        let req = client
            .post(url.join("/api/user").unwrap())
            .json(&dto)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let credential = UserCredential {
            email: "user@email.com",
            password: "wrong_pass",
        };

        let req = client
            .post(url.join("/api/auth").unwrap())
            .json(&credential)
            .build()
            .unwrap();

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
