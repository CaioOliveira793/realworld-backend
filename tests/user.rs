use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use setup::test_url;

use crate::setup::{create_client, setup_database};

mod setup;

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

    let dto = CreateUserDto {
        email: "some@email.com",
        username: "user12345",
        password: "secure:12345678",
    };

    let req = client
        .post(test_url().join("/api/user").unwrap())
        .json(&dto)
        .build()
        .unwrap();

    let res = client.execute(req).await.unwrap();

    let user: UserResponse = res.json().await.unwrap();

    assert_eq!(user.email, dto.email);
    assert_eq!(user.username, dto.username);
    assert_eq!(user.password, dto.password);
    assert_eq!(user.bio, "");
    assert_eq!(user.image, "");
}
