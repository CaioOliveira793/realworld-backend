use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use setup::test_url;

use crate::setup::{create_client, setup_database};

mod setup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResource<T> {
    pub user: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub email: String,
    pub password: String,
    pub username: String,
    pub bio: String,
    pub image: String,
}

#[tokio::test]
#[serial]
async fn insert_user() {
    setup_database();
    let client = create_client();

    let user_resource = UserResource {
        user: CreateUserDto {
            email: "some@email.com",
            username: "user12345",
            password: "secure:12345678",
        },
    };

    let req = client
        .post(test_url().join("/api/users").unwrap())
        .json(&user_resource)
        .build()
        .unwrap();

    let res = client.execute(req).await.unwrap();

    let created_user: UserResource<UserResponse> = res.json().await.unwrap();

    assert_eq!(created_user.user.email, user_resource.user.email);
    assert_eq!(created_user.user.username, user_resource.user.username);
    assert_eq!(created_user.user.password, user_resource.user.password);
    assert_eq!(created_user.user.bio, "");
    assert_eq!(created_user.user.image, "");
}
