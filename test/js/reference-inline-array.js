({
  "mirrors": [
    reference("https://a.example/x"),
    reference("https://b.example/x"),
    Uint8Array.from([0xff]),
    new Uint8Array(0),
  ],
})
