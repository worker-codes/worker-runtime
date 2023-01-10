use std::io::Cursor;

use anyhow::Result;
use quaint::{prelude::*, single::Quaint};
use rmp::{Marker, decode::RmpRead};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use wapc_codec::messagepack::serialize;
use std::io::{Read, SeekFrom};

#[derive(Deserialize, Serialize, Debug)]
pub struct QueryResult {
    columns: Vec<String>,
    #[serde(with = "serde_bytes")]
    rows: Vec<u8>,
    size: usize,
    statement: String,
    last_insert_id: Option<u64>,
    rows_affected: Option<u64>,
    time: Option<f64>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct ExecuteResult {
    statement: String,
    rows_affected: Option<u64>,
    time: Option<f64>,
}

pub async fn connected_to_database(url:String)->Result<Quaint> {
    println!("Connected to database: {}", url);
    // let conn = Quaint::new(&url).await?;
    // let conn = Quaint::new_in_memory()?;
    let conn = Quaint::new("file:///home/dallen/Codes/tikvtest/Chinook.db").await?;
    // println!("Connected to database: {}", url);

    Ok(conn)
}

pub async fn query(conn: &Quaint, query: &str, mut params: Vec<u8>) -> Result<QueryResult> {

    let params_new = read_from_msgpack(&mut params)?;

    let start = Instant::now();
    let result = conn.query_raw(query, &params_new).await?;
   

    let mut buf = Vec::new();

    let row_size = result.len();
    let last_insert_id = result.last_insert_id();
    let columns = result.columns().clone();
    let rows = result.into_iter();

    rmp::encode::write_array_len(&mut buf, row_size as u32)?;
    for row in rows {
        rmp::encode::write_array_len(&mut buf, columns.len() as u32)?;

        for column_name in &columns {
            let column = row.get(&column_name);

            if let Some(val) = column {
                write_to_msgpack(&mut buf, val)?;
            } else {
                rmp::encode::write_nil(&mut buf)?;
            }
        }
    }
    let duration = start.elapsed();
    let time = duration.as_secs() as f64;

    let query_result = QueryResult {
        columns,
        rows: buf,
        size: row_size,
        statement: query.to_string(),
        last_insert_id,
        rows_affected: Some(0),
        time: Some(time),
    };

    Ok(query_result)
}

pub async fn execute(
    conn: &Quaint,
    query: &str,
    mut params: Vec<u8>,
) -> Result<ExecuteResult> {
    let params_new = read_from_msgpack(&mut params)?;
    let start = Instant::now();
    
    let result = conn.execute_raw(query, &params_new).await?;

    let duration = start.elapsed();
    let time = duration.as_secs() as f64;

    let execute_result = ExecuteResult {
        statement: query.to_string(),
        rows_affected: Some(result),
        time: Some(time),
    };

    Ok(execute_result)
}

fn read_from_msgpack(buf: &mut Vec<u8>)-> Result<Vec<Value>> {

    let mut cur = Cursor::new(&buf);

    let mut params:Vec<Value> = vec![];
    while cur.position() < buf.len() as u64 {
        let last_pos = cur.position();
        let ext = rmp::decode::read_marker(&mut cur);

       //handle error
       let ext =match ext {
           Ok(ext) => {
               ext
           }
           Err(e) => {
            // panic!("stop here");
               return Err(anyhow::anyhow!("{:?}", e));
           }
       };

        cur.set_position(last_pos);

        match ext {
            Marker::True => {
                let val = rmp::decode::read_bool(&mut cur)?;
                params.push(Value::Boolean(Some(val)));
            }
            Marker::False => {
                let val = rmp::decode::read_bool(&mut cur)?;
                params.push(Value::Boolean(Some(val)));
            }
            Marker::I8 => {
                let val = rmp::decode::read_i8(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::I16 => {
                let val = rmp::decode::read_i16(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::I32 => {
                let val = rmp::decode::read_i32(&mut cur)?;
                params.push(Value::Int32(Some(val)));
            }
            Marker::I64 => {
                let val = rmp::decode::read_i64(&mut cur)?;
                params.push(Value::Int64(Some(val)));
            }
            Marker::U8 => {
                let val = rmp::decode::read_u8(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::U16 => {
                let val = rmp::decode::read_u16(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::U32 => {
                let val = rmp::decode::read_u32(&mut cur)?;
                params.push(Value::Int32(Some(val.try_into()?)));
            }
            Marker::U64 => {
                let val = rmp::decode::read_u64(&mut cur)?;
                params.push(Value::Int64(Some(val.try_into()?)));
            }
            Marker::F32 => {
                let val = rmp::decode::read_f32(&mut cur)?;
                params.push(Value::Float(Some(val)));
            }
            Marker::F64 => {
                let val = rmp::decode::read_f64(&mut cur)?;
                params.push(Value::Double(Some(val)));
            }
            Marker::Null => {
                let val = rmp::decode::read_nil(&mut cur)?;
                params.push(Value::Bytes(None));
            }
            Marker::FixStr(size) => {
                let val = decode_string(size, &mut cur)?;
                params.push(val.into()); 
            }
            Marker::FixPos(_size) => {
                let val = rmp::decode::read_pfix(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::FixNeg(_size) => {
                let val = rmp::decode::read_nfix(&mut cur)?;
                params.push(Value::Int32(Some(val.into())));
            }
            Marker::FixArray(_size) => {
                let val = rmp::decode::read_array_len(&mut cur)?;
                // params.push(Value::Bytes(None));
                println!("FixArray: {:?}", val );
                println!("----------------------------------")

                // let vv = vec![];
                // for _ in 0..val {
                //     // let r = read_from_msgpack(&mut cur)?;
                // }
                // // params.push(Array(Some(val)));
            }
            Marker::Bin8 => {
                let val = rmp::decode::read_bin_len(&mut cur)?;
                params.push(Value::Bytes(None));
            }
            _ => {
                println!("Unknown: {:?}", ext);
            }
        }
    }

    Ok(params)
}

fn decode_string(size: u8, cur: &mut Cursor<&&mut Vec<u8>>) ->Result<String> {

    let mut out: Vec<u8> = vec![0u8; size as usize];
    let len = rmp::decode::read_str_len(cur)?;
    let ulen = len as usize;

    if out.len() < ulen {
        return Err(anyhow::anyhow!("String too long"));
    }

    let _result = cur.read_exact_buf(&mut out[0..ulen])?;
    let val = std::str::from_utf8(&out)?;

    Ok(val.to_string())
}

fn write_to_msgpack(buf: &mut Vec<u8>, val: &Value)-> Result<()> {
    match val {
        Value::Int32(val) => {
            if let Some(val) = val {
                rmp::encode::write_i32(buf, val.clone())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Int64(val) => {
            if let Some(val) = val {
                rmp::encode::write_i64(buf, val.clone())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Float(val) => {
            if let Some(val) = val {
                rmp::encode::write_f32(buf, val.clone())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Double(val) => {
            if let Some(val) = val {
                rmp::encode::write_f64(buf, val.clone())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Text(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val)?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Enum(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val)?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Bytes(val) => {
            if let Some(val) = val {
                rmp::encode::write_bin(buf, &val)?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Boolean(val) => {
            if let Some(val) = val {
                rmp::encode::write_bool(buf, *val)?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Char(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Array(val) => {
            if let Some(val) = val {
                rmp::encode::write_array_len(buf, val.len() as u32)?;

                for v in val {
                    write_to_msgpack(buf, v)?;
                }
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        // #[cfg(feature = "bigdecimal")]
        Value::Numeric(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Json(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Xml(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        #[cfg(feature = "uuid")]
        Value::Uuid(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::DateTime(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Date(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
        Value::Time(val) => {
            if let Some(val) = val {
                rmp::encode::write_str(buf, &val.to_string())?;
            } else {
                rmp::encode::write_nil(buf)?;
            }
        }
    }

    Ok(())
}


