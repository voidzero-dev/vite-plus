which node | first | get path | print
^node
^$env.ACTIVATION_VP env list node
__ACTIVATION_COMMAND__
if (which node | first | get path) != ($env.ACTIVATION_BIN | path join node) {
    error make {msg: 'node did not resolve through the shim'}
}
^node --version
vp env list node
$env.PATH = [$env.ACTIVATION_SYSTEM $env.ACTIVATION_BIN]
^node
vp env list node
__ACTIVATION_COMMAND__
if (which node | first | get path) != ($env.ACTIVATION_BIN | path join node) {
    error make {msg: 'node did not resolve through the shim'}
}
^node --version
vp env list node
