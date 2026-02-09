Value::Object(HashMap::from([
    (
        "a".into(),
        Value::Object(HashMap::from([
            ("x".into(), Value::Integer(42.into())),
            ("y".into(), Value::String("hello".into())),
        ])),
    ),
    (
        "b".into(),
        Value::Object(HashMap::from([("z".into(), Value::Integer(42.into()))])),
    ),
]))
