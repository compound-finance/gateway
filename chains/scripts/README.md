usage: scripts to be run in the context of the repl
#in `yarn repl`
> eval(fs.readFileSync("./scripts/${my_script}.js").toString())