Value::Object(HashMap::from([
    ("items".into(), Value::Array(vec![
        Value::Object(HashMap::from([
            ("".into(), Value::String("the empty key opens this item".into())),
            ("b".into(), Value::Integer(2.into())),
        ])),
    ])),
]))
