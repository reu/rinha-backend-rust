use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Person {
    pub id: Uuid,
    #[serde(rename = "nome")]
    pub name: PersonName,
    #[serde(rename = "apelido")]
    pub nick: Nick,
    #[serde(rename = "nascimento", with = "date_format")]
    pub birth_date: Date,
    pub stack: Option<Vec<String>>,
}

#[derive(Clone, Deserialize)]
pub struct NewPerson {
    #[serde(rename = "nome")]
    pub name: PersonName,
    #[serde(rename = "apelido")]
    pub nick: Nick,
    #[serde(rename = "nascimento", with = "date_format")]
    pub birth_date: Date,
    pub stack: Option<Vec<Tech>>,
}

macro_rules! new_string_type {
    ($type:ident, max_length = $max_length:expr, error = $error_message:expr) => {
        #[derive(Clone, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        #[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
        #[cfg_attr(feature = "sqlx", sqlx(transparent))]
        pub struct $type(String);

        impl $type {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $type {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.len() <= $max_length {
                    Ok($type(value))
                } else {
                    Err($error_message)
                }
            }
        }

        impl From<$type> for String {
            fn from(value: $type) -> Self {
                value.0
            }
        }
    };
}

new_string_type!(PersonName, max_length = 100, error = "name is too big");
new_string_type!(Nick, max_length = 32, error = "nick is too big");
new_string_type!(Tech, max_length = 32, error = "tech is too big");

time::serde::format_description!(date_format, Date, "[year]-[month]-[day]");
