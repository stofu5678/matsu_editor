# The jack DocumentOpen: counts the opens in an instance variable of this file, which the resident
# process keeps for the next call of the same file.
@opens = (@opens || 0) + 1
