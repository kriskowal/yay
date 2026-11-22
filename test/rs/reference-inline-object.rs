Value::Object(HashMap::from([
    ("links".into(), Value::Object(HashMap::from([
        ("home".into(), Value::reference("https://kriskowal.com/").unwrap()),
        ("seed".into(), Value::Bytes(vec![0xff, 0x88, 0x00])),
    ]))),
]))
