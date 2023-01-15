/*---------- Imports ----------*/
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub sub: String,
    pub name: String,
    pub email: String,
}
