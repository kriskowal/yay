Value::Object(HashMap::from([
    ("color".into(), Value::Tagged("rgb".into(), Box::new(Value::Array(vec![
        Value::Integer(255.into()),
        Value::Integer(0.into()),
    ])))),
]))
