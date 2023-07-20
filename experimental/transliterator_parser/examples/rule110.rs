const ENCODE: &str = r###"
# Binary-ENCODE
# It encodes binary input (0 and 1) into a string of pairs, one pair for each character.
# At the end, it inserts the special character '%' that signifies the end.

$any = [01] ;
# backing up one char to make sure we can inser the end marker
($any) > \( 0 \, $1 | \) ;

([^]) } $ > $1 '%' ;

"###;

const DECODE: &str = r###"
# ENCODE-Binary
# It decodes pairs back to binary (0 and 1) and removes the special marker '%'.

$any = [01] ;
$next1 = \( $any \, 1 \) ;
$next0 = \( $any \, 0 \) ;

$next1 > 1 ;
$next0 > 0 ;
'%' > ;

"###;

const ONE_STEP: &str = r###"
# ENCODE-ENCODE/OneStep
# It does one iteration of the rule 110 cellular automaton,
# given the input is encoded as done in Binary-ENCODE.
# In this step, the next state will be stored in the second element of each pair.

### VARIABLE DEFINITIONS ###
$any = [01] ;
# previous state was 1
$prev1 = \( 1 \, $any \) ;
# previous state was 0
$prev0 = \( 0 \, $any \) ;
############################

####################### 00* #######################
# 000 => 0, $ special case
$prev0 { $prev0 } $ > '(0,0)' ;

# 000 => 0, ^ special case (double)
# NOT DOING THIS BECAUSE ADDING A ZERO IS WORTHLESS: # ^ { ($prev0) > '(0,0)' | $1 ;
# 001 => 1, ^ special case (double)
^ { ($prev1) > '(0,1)' | $1 ;

# 000 => 0, ^ special case
^ { $prev0 } $prev0 > '(0,0)' ;
# 001 => 1, ^ special case
^ { $prev0 } $prev1 > '(0,1)' ;
# 000 => 0
$prev0 { $prev0 } $prev0 > '(0,0)' ;
# 001 => 1
$prev0 { $prev0 } $prev1 > '(0,1)' ;
###################################################

####################### 01* #######################
# 010 => 1, $ special case
$prev0 { $prev1 } $ > '(1,1)' ;
# 011 => 1, ^ special case
^ { $prev1 } $prev1 > '(1,1)' ;
# 010 => 1, ^ special case
^ { $prev1 } $prev0 > '(1,1)' ;
# TODO: might be able to optimize these special cases down into just `^ $prev0?` (i.e., an optional)
# 011 => 1
$prev0 { $prev1 } $prev1 > '(1,1)' ;
# 010 => 1
$prev0 { $prev1 } $prev0 > '(1,1)' ;
###################################################

####################### 10* #######################
# 100 => 0, $ special case
$prev1 { $prev0 } $ > '(0,0)' ;
# 100 => 0
$prev1 { $prev0 } $prev0 > '(0,0)' ;
# 101 => 1
$prev1 { $prev0 } $prev1 > '(0,1)' ;
###################################################

####################### 11* #######################
# 110 => 1, $ special case
$prev1 { $prev1 } $ > '(1,1)' ;
# 110 => 1
$prev1 { $prev1 } $prev0 > '(1,1)' ;
# 111 => 0
$prev1 { $prev1 } $prev1 > '(1,0)' ;
###################################################

"###;

const APPLY_UPDATE: &str = r###"
# ENCODE-ENCODE/ApplyUpdate
# It moves the current state encoded as (prev_state, next_state) into (next_state, 0),

$any = [01] ;
'(' $any ',' ($any) ')' > '(' $1 ',' 0 ')' ;

"###;

const RULE_110: &str = r###"
# Binary-Binary/Rule110
# This transform applies rule 110 to the input.
# Currently it stops after a hardcoded width.

# encode
:: Binary-ENCODE ;

# run one step
^ { ([^$]*) } $ > &ENCODE-ENCODE/OneStep($1) ;

# reset to the beginning
[^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;

# move (prev_state, next_state) into (next_state, 0)
^ { ([^$]*) } $ > &ENCODE-ENCODE/ApplyUpdate($1) ;

# reset to the beginning
[^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;

# stop after pattern is 20 wide
([^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$][^$] '%') > $1 ;

# decode

:: ENCODE-Binary ;

"###;

fn main() {}