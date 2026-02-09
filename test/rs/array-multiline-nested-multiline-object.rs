Value::Array(vec![
    Value::Object(HashMap::from([
        ("x".into(), Value::Integer(10.into())),
        ("y".into(), Value::Integer(20.into())),
    ])),
    Value::Object(HashMap::from([
        ("x".into(), Value::Integer(30.into())),
        ("y".into(), Value::Integer(40.into())),
    ])),
])
