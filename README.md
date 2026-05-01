# diceroll

Roll any dice combination from the command line!

## Features

- Support an extensive dice notation
- Interactive REPL
- JSON output (`diceroll --json 4d6+4`)
- Read batches of dice rolls from standard input (`cat rolls.txt | diceroll`)
- Random seed for reproducible rolls (`--seed 42`)
- Local HTTP server (`diceroll serve`)

```
⠀⠀⠀⠀⠀⢀⣠⡴⣶⣄⡀⠀⠀⠀⠀⠀⠀
⠀⠀⢀⣤⠶⠛⠁⠀⡇⠈⠙⠷⣤⣀⠀⠀⠀
⣴⠞⠫⠥⠄⠐⠒⠲⢓⠒⠂⠠⠤⠽⠳⢦⡀
⣿⠆⠀⠀⠀⠀⡰⠁⠀⢢⠀⠀⠀⠀⢠⢻⡇
⣿⠈⡄⠀⠀⡐⠱⢲⡔⡆⠣⠀⠀⢀⠆⢸⠁
⣿⠀⠰⡀⡜⠀⠰⠥⠣⠇⠀⠱⡀⡌⠀⢸⠀
⣿⢀⠠⠻⡒⠒⠒⠒⠒⠒⠒⢒⠟⠤⡀⢸⡀
⠻⢧⣄⠀⠈⢄⠀⠀⠀⠀⡠⠊⠀⣀⣴⠟⠁
⠀⠀⠉⠛⢶⣄⡡⡀⠀⢔⣡⡴⠟⠉⠀⠀⠀
⠀⠀⠀⠀⠀⠈⠙⠿⠾⠛⠁⠀⠀⠀⠀⠀⠀
```

## Dice notation

- Use any number of dice with any number of sides: `d20`, `4d7`
- Combine several dice by adding or subtracting them, along with constants: `3d6 - 2`
- Multipliers and grouping: `d20 + (2d6+3)*2 + 5`
- Keep/drop highest/lowest: `6d4dl2` drops the 2 lowest dice, `2d20kh1` keeps the highest (advantage), `2d20kl1` keeps the lowest (disadvantage)
- Clamp dice results with minimum/maximum values: `4d6min3` treats any die below 3 as 3, `4d6max4` caps any die above 4 at 4
- Re-roll minimum results: `4d6r` rerolls any 1 until the die stops showing 1. Modifiers can be chained, e.g. `4d6rmin3kl4`
- Fate dice: `dF` or `4dF` roll values in `{-1, 0, 1}`

## Usage

Interactive REPL:

```bash
$ diceroll
>>> 4d6 + 2
4d6[1,1,4,1] + 2 = 9
>>> 2d20kh1
2d20kh1[{1},5] = 5
>>> 4d6min3
4d6min3[5,5,3,4] = 17
```

JSON output:

```bash
$ diceroll --json 3d6+4
{"total":16,"terms":[{"sign":1,"kind":"dice","count":3,"sides":6,"rolls":[5,6,1],"kept":[true,true,true],"subtotal":12},{"sign":1,"kind":"const","value":4,"subtotal":4}]}
```

Read expressions from standard input, one per line:

```bash
$ printf 'd20\n2d6dl1+6\n3d8-d4+2' | diceroll
1d20[10] = 10
2d6dl1[{3},4] + 6 = 10
3d8[8,1,3] - 1d4[4] + 2 = 10
```

Set the random seed for reproducible dice rolls:

```bash
$ diceroll --seed 42 10d20
10d20[11,11,13,9,1,9,15,17,3,1] = 90
$ diceroll --seed 42 10d20
10d20[11,11,13,9,1,9,15,17,3,1] = 90
```

Local HTTP server:

```bash
$ diceroll serve --port 8000
$ curl 'http://127.0.0.1:8000/roll?q=2d6%2B3'
2d6[4,1] + 3 = 8
$ curl -X POST --data '2d6+3' http://127.0.0.1:8000/roll
2d6[4,1] + 3 = 8
$ curl -H 'Accept: application/json' 'http://127.0.0.1:8000/roll?q=2d6%2B3'
{"total":8,"terms":[...]}
```

The server exposes two endpoints:

- `GET /roll?q=EXPR`
- `POST /roll` with the raw expression in the request body
- Plain text is returned by default
- Send `Accept: application/json` for JSON
- Encode arithmetic `+` as `%2B` in the query string, or use `curl --get --data-urlencode 'q=2d6+3'`
- The default port is `8000` and can be configured with the `--port` argument

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
