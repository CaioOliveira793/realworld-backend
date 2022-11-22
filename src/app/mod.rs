pub mod resource;
pub mod use_case;

pub mod transform {
    pub mod user {
        use crate::{
            app::resource::iam::{CreateUserDto, UserResponse},
            domain::entity::{
                iam::{User, UserState},
                Entity,
            },
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
                    created: user.created(),
                    updated: user.updated(),
                    version: user.version(),
                    username: user.username().clone(),
                    email: user.email().clone(),
                    bio: user.bio().clone(),
                    image_url: user.image_url().clone(),
                }
            }
        }
    }
}

pub mod query {}
