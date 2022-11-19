pub mod env_var {
    use lazy_static::lazy_static;

    lazy_static! {
        static ref ENV_VAR: EnvVar = load_env();
    }

    #[derive(Debug, Clone)]
    pub struct EnvVar {
        pub port: u16,
        pub token_key: String,
        pub database_host: String,
        pub database_port: u16,
        pub database_name: String,
        pub database_user: String,
        pub database_password: String,
        pub database_url: String,
    }

    macro_rules! get_env {
        ($env:literal) => {
            std::env::var($env).expect(concat!("Missing env var ", $env))
        };
    }

    fn load_env() -> EnvVar {
        let port: u16 = get_env!("PORT").parse().expect("Invalid PORT");
        let token_key = get_env!("TOKEN_KEY");
        let database_host = get_env!("DATABASE_HOST");
        let database_name = get_env!("DATABASE_NAME");
        let database_user = get_env!("DATABASE_USER");
        let database_password = get_env!("DATABASE_PASSWORD");
        let database_port: u16 = get_env!("DATABASE_PORT")
            .parse()
            .expect("Invalid DATABASE_PORT");

        let database_url = format!("postgres://{database_user}:{database_password}@{database_host}:{database_port}/{database_name}");

        EnvVar {
            port,
            token_key,
            database_host,
            database_name,
            database_password,
            database_port,
            database_user,
            database_url,
        }
    }

    pub fn get() -> &'static EnvVar {
        &ENV_VAR
    }
}
