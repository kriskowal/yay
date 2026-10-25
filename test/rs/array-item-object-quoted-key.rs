Value::Object(HashMap::from([
    ("people".into(), Value::Array(vec![
        Value::Object(HashMap::from([
            ("born".into(), Value::Integer(1815.into())),
            ("full name".into(), Value::String("Ada Lovelace".into())),
        ])),
        Value::Object(HashMap::from([
            ("born".into(), Value::Integer(1906.into())),
            ("full name".into(), Value::String("Grace Hopper".into())),
        ])),
    ])),
]))
