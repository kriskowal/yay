Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("date".into(), Box::new(Value::String("x".into())))),
    ("b".into(), Value::Tagged("t".into(), Box::new(Value::String("y".into())))),
    ("c".into(), Value::Tagged("rgb".into(), Box::new(Value::Bytes(vec![0xff])))),
    ("d".into(), Value::Tagged("rgb".into(), Box::new(Value::Array(vec![
        Value::Integer(255.into()),
        Value::Integer(136.into()),
        Value::Integer(0.into()),
    ])))),
    ("e".into(), Value::Tagged("point".into(), Box::new(Value::Object(HashMap::from([
        ("x".into(), Value::Integer(1.into())),
    ]))))),
    ("f".into(), Value::Tagged("ms".into(), Box::new(Value::Float(1.5)))),
    ("g".into(), Value::Tagged("h".into(), Box::new(Value::Integer(-3.into())))),
    ("h".into(), Value::Tagged("us".into(), Box::new(Value::Float(1000.0)))),
    ("i".into(), Value::Tagged("ms".into(), Box::new(Value::Integer(10000.into())))),
    ("j".into(), Value::Tagged("x".into(), Box::new(Value::Integer(0.into())))),
]))
