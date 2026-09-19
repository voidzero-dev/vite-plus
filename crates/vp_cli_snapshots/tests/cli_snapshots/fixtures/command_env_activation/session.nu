print '$ which node'
which node | first | get path | print
print '$ node --version'
^node --version
print ('$ ' + $env.ACTIVATION_COMMAND)
__ACTIVATION_COMMAND__
print '$ which node'
which node | first | get path | print
if (which node | first | get path) != ($env.ACTIVATION_BIN | path join node) {
    error make {msg: 'node did not resolve through the shim'}
}
print '$ node --version'
^node --version
print '$ vp env list node'
vp env list node
