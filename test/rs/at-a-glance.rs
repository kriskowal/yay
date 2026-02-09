Value::Object(HashMap::from([
    ("and-objects-too".into(), Value::Object(HashMap::from([
        ("from-their-floating-friends".into(), Value::Float(6.283185307179586)),
        ("integers-are-distinct".into(), Value::Integer(42.into())),
    ]))),
    ("arrays".into(), Value::Array(vec![
        Value::String("may".into()),
        Value::String("have".into()),
        Value::String("many".into()),
        Value::String("values".into()),
    ])),
    ("block".into(), Value::Object(HashMap::from([
        ("array".into(), Value::Array(vec![
            Value::String("But".into()),
            Value::String("this".into()),
            Value::String("one's".into()),
        ])),
        ("bytes".into(), Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff, 0xfe, 0xfa, 0xca, 0xde])),
        ("object".into(), Value::Object(HashMap::from([
            ("mine".into(), Value::Null),
        ]))),
        ("string".into(), Value::String("This is a string.\nThere are many like it.\n".into())),
    ]))),
    ("concatenated".into(), Value::String("I'm not dead yet. I feel happy!".into())),
    ("inline".into(), Value::Object(HashMap::from([
        ("array".into(), Value::Array(vec![
            Value::Float(f64::INFINITY),
            Value::Float(f64::NEG_INFINITY),
            Value::Float(f64::NAN),
        ])),
        ("bytes".into(), Value::Bytes(vec![0xf3, 0x3d, 0xfa, 0xce])),
        ("object".into(), Value::Object(HashMap::from([
            ("bigint".into(), Value::Integer(1.into())),
            ("float64".into(), Value::Float(2.0)),
        ]))),
        ("string".into(), Value::String("is concise".into())),
    ]))),
    ("name with spaces".into(), Value::String("works too".into())),
    ("roses-are-red".into(), Value::Bool(true)),
    ("unicode-code-point".into(), Value::String("😀".into())),
    ("violets-are-blue".into(), Value::Bool(false)),
]))
