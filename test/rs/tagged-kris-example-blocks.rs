Value::Object(HashMap::from([
    ("color".into(), Value::Tagged("rgb".into(), Box::new(Value::Array(vec![
        Value::Integer(255.into()),
        Value::Integer(136.into()),
        Value::Integer(0.into()),
    ])))),
    ("items".into(), Value::Array(vec![
        Value::Tagged("match:splitRecord".into(), Box::new(Value::Object(HashMap::from([
            ("name".into(), Value::String("x".into())),
        ])))),
    ])),
]))
