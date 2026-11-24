Value::Object(HashMap::from([
    ("at".into(), Value::Tagged("point".into(), Box::new(Value::Object(HashMap::from([
        ("x".into(), Value::Integer(1.into())),
        ("y".into(), Value::Integer(2.into())),
    ]))))),
    ("next".into(), Value::Integer(3.into())),
]))
