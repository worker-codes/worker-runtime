use crate::error::AnyError;
use crate::error::type_error;


fn get_kvstorage(state: &mut OpState, persistent: bool) -> Result<&Connection, AnyError> {

    Ok(conn)
}



pub fn kvstorage_put(
    state: &mut OpState,
    key: String,
    value: String,
    persistent: bool,
) -> Result<(), AnyError> {
    let conn = get_kvstorage(state, persistent)?;

    let txn_client = TransactionClient::new(vec!["127.0.0.1:2379"]).await?;
    let mut txn = txn_client.begin_optimistic().await?;
    txn.put("key".to_owned(), "value".to_owned()).await?;

    
    txn.commit().await?;

    Ok(())
}

pub fn kvstorage_get(
    state: &mut OpState,
    key_name: String,
    persistent: bool,
) -> Result<Option<String>, AnyError> {
    let conn = get_kvstorage(state, persistent)?;

    let mut stmt = conn.prepare_cached("SELECT value FROM data WHERE key = ?")?;
    let val = stmt
        .query_row(params![key_name], |row| row.get(0))
        .optional()?;

    Ok(val)
}

pub fn kvstorage_delete(
    state: &mut OpState,
    key_name: String,
    persistent: bool,
) -> Result<(), AnyError> {
    let conn = get_kvstorage(state, persistent)?;

    let mut stmt = conn.prepare_cached("DELETE FROM data WHERE key = ?")?;
    stmt.execute(params![key_name])?;

    Ok(())
}

pub fn kvstorage_clear(state: &mut OpState, persistent: bool) -> Result<(), AnyError> {
    let conn = get_kvstorage(state, persistent)?;

    let mut stmt = conn.prepare_cached("DELETE FROM data")?;
    stmt.execute(params![])?;

    Ok(())
}

pub fn kvstorage_iterate_keys(
    state: &mut OpState,
    persistent: bool,
) -> Result<Vec<String>, AnyError> {
    let conn = get_kvstorage(state, persistent)?;

    let mut stmt = conn.prepare_cached("SELECT key FROM data")?;
    let keys = stmt
        .query_map(params![], |row| row.get::<_, String>(0))?
        .map(|r| r.unwrap())
        .collect();

    Ok(keys)
}
