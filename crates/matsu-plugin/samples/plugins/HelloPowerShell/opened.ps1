# The jack DocumentOpen: counts the opens in a global variable, which the resident process keeps for
# the next call.
$global:Opens = [int]$global:Opens + 1
