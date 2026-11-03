Value::Object(HashMap::from([
    ("a".into(), Value::Array(vec![
        Value::Tagged("s".into(), Box::new(Value::Integer(10.into()))),
        Value::Tagged("x".into(), Box::new(Value::String("ff".into()))),
        Value::Tagged("a, b".into(), Box::new(Value::Array(vec![Value::Integer(1.into())]))),
        Value::Tagged("t".into(), Box::new(Value::Bool(true))),
    ])),
    ("b".into(), Value::Object(HashMap::from([
        ("c".into(), Value::Tagged("rgb".into(), Box::new(Value::Bytes(vec![0xff, 0x88, 0x00])))),
        ("d".into(), Value::Tagged("m".into(), Box::new(Value::Integer(2.into())))),
        ("e".into(), Value::Tagged("x}y".into(), Box::new(Value::String("v".into())))),
        ("f".into(), Value::Tagged(")".into(), Box::new(Value::Integer(1.into())))),
    ]))),
]))
