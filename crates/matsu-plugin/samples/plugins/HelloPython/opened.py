# The jack DocumentOpen: counts the opens in the namespace of this file, which the resident process
# keeps for the next call of the same file.
opens = globals().get("opens", 0) + 1
