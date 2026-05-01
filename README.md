# diceroll

Roll any dice combination from the command line!

## Features

- Support an extensive dice notation
- Interactive REPL
- JSON output (`diceroll --json 4d6+4`)
- Read batches of dice rolls from standard input (`cat rolls.txt | diceroll`)
- Random seed for reproducible rolls (`--seed 42`)

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

## Usage

Interactive REPL:

```bash
$ diceroll
>>> 4d6 + 2
4d6[1,1,4,1] + 2 = 9
>>> 2d20kh1
2d20kh1[{1},5] = 5
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
