const ENCODE: &str = r###"
# Binary-ENCODE
# It encodes binary input (0 and 1) into a string of pairs, one pair for each character.
# At the end, it inserts the special character '%' that signifies the end.

$any = [01] ;
# backing up one char to make sure we can inser the end marker
($any) > '(' $1 ',' 0 | ')' ;

([^]) } [$] > $1 '%' ;

"###;

const DECODE: &str = r###"
# ENCODE-Binary
# It decodes pairs back to binary (0 and 1) and removes the special marker '%'.

$any = [01] ;
$next1 = '(' $any ',' 1 ')' ;
$next0 = '(' $any ',' 0 ')' ;

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
$prev1 = '(' 1 ',' $any ')' ;
# previous state was 0
$prev0 = '(' 0 ',' $any ')' ;
############################

####################### 00* #######################
# 000 => 0, $ special case
$prev0 { $prev0 } [$] > '(0,0)' ;

# 000 => 0, ^ special case (double)
# NOT DOING THIS BECAUSE ADDING A ZERO IS WORTHLESS: # ^ { ($prev0) > '(0,0)' | $1 ;
# 001 => 1, ^ special case (double)
[$] { ($prev1) > '(0,1)' | $1 ;

# 000 => 0, ^ special case
[$] { $prev0 } $prev0 > '(0,0)' ;
# 001 => 1, ^ special case
[$] { $prev0 } $prev1 > '(0,1)' ;
# 000 => 0
$prev0 { $prev0 } $prev0 > '(0,0)' ;
# 001 => 1
$prev0 { $prev0 } $prev1 > '(0,1)' ;
###################################################

####################### 01* #######################
# 010 => 1, $ special case
$prev0 { $prev1 } [$] > '(1,1)' ;
# 011 => 1, ^ special case
[$] { $prev1 } $prev1 > '(1,1)' ;
# 010 => 1, ^ special case
[$] { $prev1 } $prev0 > '(1,1)' ;
# TODO: might be able to optimize these special cases down into just `^ $prev0?` (i.e., an optional)
# 011 => 1
$prev0 { $prev1 } $prev1 > '(1,1)' ;
# 010 => 1
$prev0 { $prev1 } $prev0 > '(1,1)' ;
###################################################

####################### 10* #######################
# 100 => 0, $ special case
$prev1 { $prev0 } [$] > '(0,0)' ;
# 100 => 0
$prev1 { $prev0 } $prev0 > '(0,0)' ;
# 101 => 1
$prev1 { $prev0 } $prev1 > '(0,1)' ;
###################################################

####################### 11* #######################
# 110 => 1, $ special case
$prev1 { $prev1 } [$] > '(1,1)' ;
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

$pair = '(' [01] ',' [01] ')' ;

# run one step
[$] { ([^%$]*) } '%' [$] > | &ENCODE-ENCODE/OneStep($1) ;

# reset to the beginning
# [^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;

# move (prev_state, next_state) into (next_state, 0)
[$] { ([^%$]*) } '%' [$] > | &ENCODE-ENCODE/ApplyUpdate($1) ;

# reset to the beginning
# [^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;

# stop after pattern is 20 wide
($pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair '%') > $1 ;

# decode

:: ENCODE-Binary ;

"###;

fn main() {
    // print these to copy paste into java
    println!("String rulesEncode = {:?};", ENCODE);
    println!("String rulesDecode = {:?};", DECODE);
    println!("String rulesOneStep = {:?};", ONE_STEP);
    println!("String rulesApplyUpdate = {:?};", APPLY_UPDATE);
    println!("String rulesRule110 = {:?};", RULE_110);
}

/*

String rulesEncode = "\n# Binary-ENCODE\n# It encodes binary input (0 and 1) into a string of pairs, one pair for each character.\n# At the end, it inserts the special character '%' that signifies the end.\n\n$any = [01] ;\n# backing up one char to make sure we can inser the end marker\n($any) > '(' $1 ',' 0 | ')' ;\n\n([^]) } [$] > $1 '%' ;\n\n";
String rulesDecode = "\n# ENCODE-Binary\n# It decodes pairs back to binary (0 and 1) and removes the special marker '%'.\n\n$any = [01] ;\n$next1 = '(' $any ',' 1 ')' ;\n$next0 = '(' $any ',' 0 ')' ;\n\n$next1 > 1 ;\n$next0 > 0 ;\n'%' > ;\n\n";
String rulesOneStep = "\n# ENCODE-ENCODE/OneStep\n# It does one iteration of the rule 110 cellular automaton,\n# given the input is encoded as done in Binary-ENCODE.\n# In this step, the next state will be stored in the second element of each pair.\n\n### VARIABLE DEFINITIONS ###\n$any = [01] ;\n# previous state was 1\n$prev1 = '(' 1 ',' $any ')' ;\n# previous state was 0\n$prev0 = '(' 0 ',' $any ')' ;\n############################\n\n####################### 00* #######################\n# 000 => 0, $ special case\n$prev0 { $prev0 } [$] > '(0,0)' ;\n\n# 000 => 0, ^ special case (double)\n# NOT DOING THIS BECAUSE ADDING A ZERO IS WORTHLESS: # ^ { ($prev0) > '(0,0)' | $1 ;\n# 001 => 1, ^ special case (double)\n[$] { ($prev1) > '(0,1)' | $1 ;\n\n# 000 => 0, ^ special case\n[$] { $prev0 } $prev0 > '(0,0)' ;\n# 001 => 1, ^ special case\n[$] { $prev0 } $prev1 > '(0,1)' ;\n# 000 => 0\n$prev0 { $prev0 } $prev0 > '(0,0)' ;\n# 001 => 1\n$prev0 { $prev0 } $prev1 > '(0,1)' ;\n###################################################\n\n####################### 01* #######################\n# 010 => 1, $ special case\n$prev0 { $prev1 } [$] > '(1,1)' ;\n# 011 => 1, ^ special case\n[$] { $prev1 } $prev1 > '(1,1)' ;\n# 010 => 1, ^ special case\n[$] { $prev1 } $prev0 > '(1,1)' ;\n# TODO: might be able to optimize these special cases down into just `^ $prev0?` (i.e., an optional)\n# 011 => 1\n$prev0 { $prev1 } $prev1 > '(1,1)' ;\n# 010 => 1\n$prev0 { $prev1 } $prev0 > '(1,1)' ;\n###################################################\n\n####################### 10* #######################\n# 100 => 0, $ special case\n$prev1 { $prev0 } [$] > '(0,0)' ;\n# 100 => 0\n$prev1 { $prev0 } $prev0 > '(0,0)' ;\n# 101 => 1\n$prev1 { $prev0 } $prev1 > '(0,1)' ;\n###################################################\n\n####################### 11* #######################\n# 110 => 1, $ special case\n$prev1 { $prev1 } [$] > '(1,1)' ;\n# 110 => 1\n$prev1 { $prev1 } $prev0 > '(1,1)' ;\n# 111 => 0\n$prev1 { $prev1 } $prev1 > '(1,0)' ;\n###################################################\n\n";
String rulesApplyUpdate = "\n# ENCODE-ENCODE/ApplyUpdate\n# It moves the current state encoded as (prev_state, next_state) into (next_state, 0),\n\n$any = [01] ;\n'(' $any ',' ($any) ')' > '(' $1 ',' 0 ')' ;\n\n";
String rulesRule110 = "\n# Binary-Binary/Rule110\n# This transform applies rule 110 to the input.\n# Currently it stops after a hardcoded width.\n\n# encode\n:: Binary-ENCODE ;\n\n$pair = '(' [01] ',' [01] ')' ;\n\n# run one step\n[$] { ([^%$]*) } '%' [$] > | &ENCODE-ENCODE/OneStep($1) ;\n\n# reset to the beginning\n# [^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;\n\n# move (prev_state, next_state) into (next_state, 0)\n[$] { ([^%$]*) } '%' [$] > | &ENCODE-ENCODE/ApplyUpdate($1) ;\n\n# reset to the beginning\n# [^]* { '%' > | @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ '%' ;\n\n# stop after pattern is 20 wide\n($pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair$pair '%') > $1 ;\n\n# decode\n\n:: ENCODE-Binary ;\n\n";

 */