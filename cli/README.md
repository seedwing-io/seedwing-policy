## Seedwing Policy CLI

The Seedwing Policy CLI provides access to the functionality of the Seedwing
Policy Engine without requiring access to a running Policy Server.

## Examples

### Basic rule usage
This first example show the simplest possible rule.

First we need to generate a rule:
```console
$ echo "pattern nr = 18" > nr-rule.dog
```
And we also need have an input file:
```console
$ echo 18 > input.txt
```
```console
$ cargo r -q --bin seedwing-policy-cli -- \
     --policy nr-rule.dog eval \
     --input input.txt \
     --name nr-rule::nr
evaluate pattern: nr-rule::nr
Pattern: nr-rule::nr
Satisfied: true
Value:
  18
Rationale:
  primordial(true)

ok!
```

### JSON input format
First we create our rule, and save it in a file named `json-rule.dog`:
```console
$ echo "pattern nr = {
   nr: integer,
}" > json-rule.dog
```

And then we create an input file, and save it in a file named `input.json`:
```console
$ echo "{
  "nr": 18
}" > input.json
```
And we can run the `eval` command:
```console
$ cargo r -q --bin seedwing-policy-cli -- \
     --policy json-rule.dog \
     eval \
     --input input.json \
     --name json-rule::nr

### YAML input format
First we create our rule, and save it in a file named `yaml-rule.dog`:
```console
$ echo "pattern nr = {
   nr: integer,
}" > yaml-rule.dog
```

And then we create an input file, and save it in a file named `input.yaml`:
```console
$ echo "---
  nr: 18" > input.yaml
```

And then we specify the `-t` option to the `eval` command and specify the format
as `yaml`:
```console
$ cargo r -q --bin seedwing-policy-cli -- \
     --policy yaml-rule.dog \
     eval -t yaml \
     --input input.yaml \
     --name yaml-rule::nr
evaluate pattern: yaml-rule::nr
Pattern: yaml-rule::nr
Satisfied: true
Value:
  nr: <<integer>>
Rationale:
  field: nr
    Pattern: <none>
    Satisfied: true
    Value:
      18
    Rationale:
      primordial(true)

ok!
```

