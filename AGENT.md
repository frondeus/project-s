0. DO NOT run `cargo run`. It won't work, it is ONLY for LSP.
If you need to test if it works, use snapshot test or write unit test.

1. After you make a change, make sure it is passing

```
cargo xtask llm
```

I will inform you if there are some tests not passing because of you or not in the beginning of your work.

2. Make sure you understand how language behaves and what is the syntax by looking at `./docs/reference`. If in doubt, stop and ask human.

3. Do not modify journal or references. Those are for human only.

4. This project is using snapshot tests in form of markdown files.
You can use `./docs/llm.md` file for your purposes.

The basic syntax is:

```s
input code
```

And then codeblocks with different "name" are used to assert certain aspects of the language.

The most valuable is probably `eval`.
Then, `processed` - returns SEXPs after AST transformations (passes)
Then, `graphviz` - for generating type inference graphs.
Then, `traces` for runtime traces and `type-traces` for inferer only traces.

To see all possible options grep `test_runner::test_snapshots`. We use snapshots also to
temporairly (as in journal) snapshot compiler/runtime internals.

Snapshots can be also ignored with ` ignore` flag.
Example

```eval ignore
foo
```

5. Basic architecture:
* tree-sitter-s has CST grammar in `grammar.js`.
* then CST is transformed to "AST" -> however it is not very abstract for now.
* "AST" is preprocessed, then typechecked, then post-processed
* Final AST is passed to the Runtime that is tree walking.

main.rs only is for LSP implementation. There is no entrypoint for actually using that language! And there wont be in near future.
There is no REPL (yet).

Our source of truth are snapshot tests mostly in `docs/reference`.
There are some features that were introduced but since we got typechecker are no longer valid. Keep them as they are unless asked.

We are still in the discovery phase of what we want.

* zed folder contains integration with Zed Editor, syntax highlighting and LSP client.
* vscode has basic (and no longer maintained, i moved to Zed) plugin
for vscode based editors. You can ignore it.
* xtask is for humans, except for `cargo xtask llm` but you do not modify it, ever.
* Ignore `test-runner`.
