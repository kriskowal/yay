Value::Object(HashMap::from([
    ("parrot".into(), Value::Object(HashMap::from([
        ("condition".into(), Value::String("No, no, it's just resting!\n".into())),
        ("remarks".into(), Value::Array(vec![
            Value::String("Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n".into()),
            Value::String("It's probably pining for the fjords.\nLovely plumage.\n".into()),
        ])),
    ]))),
]))
