Value::Object(HashMap::from([
    ("color".into(), Value::Tagged("rgba".into(), Box::new(Value::Bytes(vec![0xff, 0x88, 0x00, 0xff])))),
    ("count".into(), Value::Tagged("s".into(), Box::new(Value::Integer(10.into())))),
    ("odd".into(), Value::Tagged("a)b".into(), Box::new(Value::Integer(3.into())))),
    ("sku".into(), Value::Tagged("com.example/sku".into(), Box::new(Value::String("X-1".into())))),
    ("when".into(), Value::Tagged("utc".into(), Box::new(Value::Tagged("datetime".into(), Box::new(Value::String("2026-11-03T05:00:00Z".into())))))),
]))
