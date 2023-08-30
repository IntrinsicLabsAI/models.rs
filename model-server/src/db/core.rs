
// Include the builtin connection from the pool.
// We should ensure that once we run the connection, we have access to the
// underlying connection that's shared across threads.


// pub trait DataManager {
//     fn get_registered_models(&self) -> Vec<RegisteredModel>;
// }

// pub struct SqliteDataManager {
//     connection: rusqlite::Connection,
// }

// // RegisteredModel for some things here
// impl DataManager for SqliteDataManager {
//     fn get_registered_models(&self) -> Vec<RegisteredModel> {
//         todo!()
//     }
// }