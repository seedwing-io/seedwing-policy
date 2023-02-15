## Seedwing Policy CLI

The Seedwing Policy CLI provides access to the functionality of the Seedwing
Policy Engine without requiring access to a running Policy Server.

### Example usage
First we need to generate a rule:
```console
$ echo "pattern nr = integer" > nr-rule.dog
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
Type: nr-rule::nr
Satisfied: true
Value:
  18
Rationale:
  primordial(true)

ok!
```

