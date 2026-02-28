use crate::{connection::*, store::Store};

pub async fn execute_command(args: Frame, store: Store) -> Result<Frame, Error> {
    let Frame::Array(frames) = args else {
        return Ok(Frame::Error(
            "ERR PROTOCOL ERROR: expected array".to_string(),
        ));
    };

    let Some(first) = frames.first() else {
        return Ok(Frame::Error(
            "ERR PROTOCOL ERROR: expected array with at least one element".to_string(),
        ));
    };

    let Some(cmd_name) = to_string(first) else {
        return Ok(Frame::Error(
            "ERR command name must be a string ".to_string(),
        ));
    };

    match cmd_name.to_uppercase().as_str() {
        "SET" => set(&Frame::Array(frames), &store),
        "GET" => get(&Frame::Array(frames), &store),
        "HSET" => hset(&Frame::Array(frames), &store),
        "HGET" => hget(&Frame::Array(frames), &store),
        "PING" => Ok(Frame::Simple("PONG".to_string())),
        _ => Ok(Frame::Error(format!("ERR unknown command '{}'", cmd_name))),
    }
}

fn set(args: &Frame, store: &Store) -> Result<Frame, Error> {
    let Frame::Array(frames) = args else {
        return Ok(Frame::Error("Err invalid args ".to_string()));
    };

    if frames.len() != 3 {
        return Ok(Frame::Error(
            "Err wrong number of arguments for 'set' command".to_string(),
        ));
    }

    let Some(key) = to_string(&frames[1]) else {
        return Ok(Frame::Error("ERR key must be a string".to_string()));
    };

    store.set(key, frames[2].clone());
    Ok(Frame::Simple("OK".to_string()))
}

fn get(args: &Frame, store: &Store) -> Result<Frame, Error> {
    let Frame::Array(frames) = args else {
        return Ok(Frame::Error("ERR Invalid args".to_string()));
    };
    if frames.len() != 2 {
        return Ok(Frame::Error(
            "ERR wrong number of arguments for 'get' command".to_string(),
        ));
    }

    let Some(key) = to_string(&frames[1]) else {
        return Ok(Frame::Error("ERR key must be a string".to_string()));
    };

    Ok(store.get(key).unwrap_or(Frame::Null))
}

fn hset(args: &Frame, store: &Store) -> Result<Frame, Error> {
    let Frame::Array(frames) = args else {
        return Ok(Frame::Error("Err Invalid args".to_string()));
    };
    if frames.len() != 4 {
        return Ok(Frame::Error(
            "Err wrong number of arguments to 'hset' command".to_string(),
        ));
    }

    let Some(key) = to_string(&frames[1]) else {
        return Ok(Frame::Error("ERR key must be a string".to_string()));
    };
    let Some(field) = to_string(&frames[2]) else {
        return Ok(Frame::Error("ERR field must be a string".to_string()));
    };

    if store
        .hset(key.clone(), field.clone(), frames[3].clone())
        .is_err()
    {
        return Ok(Frame::Error("ERR invalid field or key".to_string()));
    }

    Ok(Frame::Simple("OK".to_string()))
}

fn hget(args: &Frame, store: &Store) -> Result<Frame, Error> {
    let Frame::Array(frames) = args else {
        return Ok(Frame::Error("Err Invalid args".to_string()));
    };
    if frames.len() != 3 {
        return Ok(Frame::Error(
            "Err wrong number of arguments for 'hget' command".to_string(),
        ));
    }

    let Some(key) = to_string(&frames[1]) else {
        return Ok(Frame::Error("ERR key must be a string".to_string()));
    };
    let Some(field) = to_string(&frames[2]) else {
        return Ok(Frame::Error("ERR field must be a string".to_string()));
    };

    match store.hget(&key, &field) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Ok(Frame::Null),
        Err(e) => Ok(Frame::Error(format!("{:?}", e))),
    }
}

fn to_string(frame: &Frame) -> Option<String> {
    match frame {
        Frame::Simple(s) => Some(s.clone()),
        Frame::Bulk(b) => Some(String::from_utf8_lossy(b).to_string()),
        _ => None,
    }
}
