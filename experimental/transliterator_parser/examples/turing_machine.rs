const ENCODE: &str = r###"

# Any-TMEncoding ;
# Encodes input code points into the turing machine tape.

$blank = '0' ;

$c = [^$] ;

# [$] { ($c) > '(SYM_BEGIN,state)' | $1 ;

# first input zero is in the begin state
[$] { ($c) > '(' $1 ',' currA ')' ;

($c) > '(' $1 ',' state ')' ;


"###;

const DECODE: &str = r###"

# TMEncoding-Any ;
# Decodes the turing machine tape into raw numbers.

$c = [^$] ;
$anyState = [a-zA-Z]+;

'(' ($c) ',' $anyState ')' > $1 ;


"###;

const ONE_STEP: &str = r###"
# TMEncoding-TMEncoding/OneStep

:: TMEncoding-TMEncoding/ExtendTape ;

$any = [01] ;
$anyState = [a-zA-Z]+;

# turing machine steps described as:
# input,state => write,move_dir,new_state

# 0,A => 1,R,B
'(' 0 ',' currA ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' currB ')' ;

# 1,A => 1,L,C
'(' ($any) ',' $anyState ')' '(' 1 ',' currA ')' > '(' $1 ',' currC ')' '(' 1 ',' x ')' ;

# 0,B => 1,L,A
'(' ($any) ',' $anyState ')' '(' 0 ',' currB ')' > '(' $1 ',' currA ')' '(' 1 ',' x ')' ;

# 1,B => 1,R,B
'(' 1 ',' currB ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' currB ')' ;

# 0,C => 1,L,B
'(' ($any) ',' $anyState ')' '(' 0 ',' currC ')' > '(' $1 ',' currB ')' '(' 1 ',' x ')' ;

# 1,C => 1,R,HALT
'(' 1 ',' currC ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' HALT ')' ;

"###;

const EXTEND: &str = r###"
# TMEncoding-TMEncoding/ExtendTape

$c = [^$];

$empty_sym = 0 ;

[$] { ( '(' $c ',' curr [a-zA-Z]+ ')' ) > | '(' $empty_sym ',' state ')' $1 ;
( '(' $c ',' curr [a-zA-Z]+ ')' ) } [$] > $1 '(' $empty_sym ',' state ')' ;


"###;

const TURING_MACHINE: &str = r###"
# Any-Any/TM ;

:: Any-TMEncoding ;

$any = [01] ;
$anyState = [a-zA-Z]+;

# stop if halt exists
[$] { ([^H$]* HALT [^$]*) } [$] > $1 ;

# simulate a step
[$] { ([^$]+) } [$] > | &TMEncoding-TMEncoding/OneStep($1) ;

:: TMEncoding-Any;

"###;

fn main() {
    // print these to copy paste into java
    println!("String rulesEncode = {:?};", ENCODE);
    println!("String rulesDecode = {:?};", DECODE);
    println!("String rulesExtend = {:?};", EXTEND);
    println!("String rulesOneStep = {:?};", ONE_STEP);
    println!("String rulesTM = {:?};", TURING_MACHINE);

    println!(r#"

Transliterator encode = Transliterator.createFromRules("Any-TMEncoding", rulesEncode, Transliterator.FORWARD);
Transliterator decode = Transliterator.createFromRules("TMEncoding-Any", rulesDecode, Transliterator.FORWARD);
Transliterator extend = Transliterator.createFromRules("TMEncoding-TMEncoding/ExtendTape", rulesExtend, Transliterator.FORWARD);

Transliterator.registerInstance(encode);
Transliterator.registerInstance(decode);
Transliterator.registerInstance(extend);

Transliterator oneStep = Transliterator.createFromRules("TMEncoding-TMEncoding/OneStep", rulesOneStep, Transliterator.FORWARD);

Transliterator.registerInstance(oneStep);

Transliterator tm = Transliterator.createFromRules("Any-Any/TM", rulesTM, Transliterator.FORWARD);



    "#);
}

/*

String rulesEncode = "\n\n# Any-TMEncoding ;\n# Encodes input code points into the turing machine tape.\n\n$blank = '0' ;\n\n$c = [^$] ;\n\n# [$] { ($c) > '(SYM_BEGIN,state)' | $1 ;\n\n# first input zero is in the begin state\n[$] { ($c) > '(' $1 ',' currA ')' ;\n\n($c) > '(' $1 ',' state ')' ;\n\n\n";
String rulesDecode = "\n\n# TMEncoding-Any ;\n# Decodes the turing machine tape into raw numbers.\n\n$c = [^$] ;\n$anyState = [a-zA-Z]+;\n\n'(' ($c) ',' $anyState ')' > $1 ;\n\n\n";
String rulesExtend = "\n# TMEncoding-TMEncoding/ExtendTape\n\n$c = [^$];\n\n$empty_sym = 0 ;\n\n[$] { ( '(' $c ',' curr [a-zA-Z]+ ')' ) > | '(' $empty_sym ',' state ')' $1 ;\n( '(' $c ',' curr [a-zA-Z]+ ')' ) } [$] > $1 '(' $empty_sym ',' state ')' ;\n\n\n";
String rulesOneStep = "\n# TMEncoding-TMEncoding/OneStep\n\n:: TMEncoding-TMEncoding/ExtendTape ;\n\n$any = [01] ;\n$anyState = [a-zA-Z]+;\n\n# turing machine steps described as:\n# input,state => write,move_dir,new_state\n\n# 0,A => 1,R,B\n'(' 0 ',' currA ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' currB ')' ;\n\n# 1,A => 1,L,C\n'(' ($any) ',' $anyState ')' '(' 1 ',' currA ')' > '(' $1 ',' currC ')' '(' 1 ',' x ')' ;\n\n# 0,B => 1,L,A\n'(' ($any) ',' $anyState ')' '(' 0 ',' currB ')' > '(' $1 ',' currA ')' '(' 1 ',' x ')' ;\n\n# 1,B => 1,R,B\n'(' 1 ',' currB ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' currB ')' ;\n\n# 0,C => 1,L,B\n'(' ($any) ',' $anyState ')' '(' 0 ',' currC ')' > '(' $1 ',' currB ')' '(' 1 ',' x ')' ;\n\n# 1,C => 1,R,HALT\n'(' 1 ',' currC ')' '(' ($any) ',' $anyState ')' > '(' 1 ',' x ')' '(' $1 ',' HALT ')' ;\n\n";
String rulesTM = "\n# Any-Any/TM ;\n\n:: Any-TMEncoding ;\n\n$any = [01] ;\n$anyState = [a-zA-Z]+;\n\n# stop if halt exists\n[$] { ([^H$]* HALT [^$]*) } [$] > $1 ;\n\n# simulate a step\n[$] { ([^$]+) } [$] > | &TMEncoding-TMEncoding/OneStep($1) ;\n\n:: TMEncoding-Any;\n\n";


Transliterator encode = Transliterator.createFromRules("Any-TMEncoding", rulesEncode, Transliterator.FORWARD);
Transliterator decode = Transliterator.createFromRules("TMEncoding-Any", rulesDecode, Transliterator.FORWARD);
Transliterator extend = Transliterator.createFromRules("TMEncoding-TMEncoding/ExtendTape", rulesExtend, Transliterator.FORWARD);

Transliterator.registerInstance(encode);
Transliterator.registerInstance(decode);
Transliterator.registerInstance(extend);

Transliterator oneStep = Transliterator.createFromRules("TMEncoding-TMEncoding/OneStep", rulesOneStep, Transliterator.FORWARD);

Transliterator.registerInstance(oneStep);

Transliterator tm = Transliterator.createFromRules("Any-Any/TM", rulesTM, Transliterator.FORWARD);





 */