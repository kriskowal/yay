Value::Object(HashMap::from([
    ("steps".into(), Value::Array(vec![
        Value::Object(HashMap::from([
            ("run".into(), Value::Object(HashMap::from([
                ("command".into(), Value::String("make".into())),
            ]))),
        ])),
        Value::String("a plain item after an item holding an object".into()),
    ])),
]))
