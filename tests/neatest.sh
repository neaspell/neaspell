#!/bin/bash
# Runner and converter for internal and external test cases.
INT_TEST_DIR=$(dirname "$0")
# echo "Internal test directory is $INT_TEST_DIR"
if [[ "$1" == "--int" ]] ; then
    INTERNAL="1"
    shift 1
fi
if [[ "$1" == "--ext" ]] ; then
    EXTERNAL="1"
    shift 1
fi
if [[ "$1" == "--ed" && -n "$2" ]] ; then
    EXT_TEST_DIR="$2"
    shift 2
fi
if [[ "$1" == "--ec" && -n "$2" ]] ; then
    EXT_TEST_CMD="$2"
    shift 2
fi
if [[ "$1" == "--ic" && -n "$2" ]] ; then
    INT_TEST_CMD="$2"
    shift 2
fi
if [ "$#" -eq 0 ]; then
    echo "Usage [--int] [-ext] [--ed EXT_TEST_DIR] [--ec EXT_TEST_CMD] [--ic INT_TEST_CMD] TEST_CASE_NAME..."
    echo "--int: convert internal test case to external text case to EXT_TEST_DIR"
    echo "--ext: convert external test case to internal text case from EXT_TEST_DIR"
    echo "The internal test cases are in the directory of this script, neatest.sh."
    echo "An internal test case is in a file with extension neadic."
    echo "An external test case is in four files with extensions aff, dic, good, wrong."
    echo "--ed EXT_TEST_DIR: external test directory"
    echo "--it INT_TEST_CMD: run internal test command with the initial arguments"
    echo "--et EXT_TEST_CMD: run external test command with the initial arguments"
    echo "TEST_CASE_NAME is the test file name, (directory or extension are ignored)"
    echo "The variables EXT_TEST_DIR, INT_TEST_CMD, EXT_TEST_CMD can be used instead of arguments."
    echo "Limitation: The script doesn't change the encoding."
    exit 1
fi

for TEST_CASE in $* ; do
    BASE_NAME=$(basename "$TEST_CASE")
    BASE_NAME="${BASE_NAME%.*}"
    INT_NAME="$INT_TEST_DIR/$BASE_NAME.neadic"
    EXT_BASE="$EXT_TEST_DIR/$BASE_NAME"
    #echo "BASE_NAME=$BASE_NAME"
    #echo "INT_NAME=$INT_NAME"
    #echo "EXT_BASE=$EXT_BASE"
    if [ -n "$EXTERNAL" ] ; then
        #echo "Converting external test case to $INT_NAME"
        if [ -f "$EXT_BASE.aff" ] ; then
            cat $EXT_BASE.aff > $INT_NAME
            #echo "    included $EXT_BASE.aff"
        else
            echo "Missing $EXT_BASE.aff, exiting"
            exit 1
        fi
        if [ -f "$EXT_BASE.dic" ] ; then
            echo "NEA DIC {" >> $INT_NAME
            tail -n +2 $EXT_BASE.dic | sed 's/^/    /' >> $INT_NAME
            echo "}" >> $INT_NAME
            #echo "    included $EXT_BASE.dic"
        else
            echo "Missing $EXT_BASE.dic, exiting"
            exit 1
        fi
        if [ -f "$EXT_BASE.good" ] ; then
            echo "NEA TESTGOODWORDS {" >> $INT_NAME
            cat "$EXT_BASE.good" | sed 's/^/    /' >> $INT_NAME
            echo "}" >> $INT_NAME
            #echo "    included $EXT_BASE.good"
        fi
        if [ -f "$EXT_BASE.wrong" ] ; then
            echo "NEA TESTBADWORDS {" >> $INT_NAME
            cat "$EXT_BASE.wrong" | sed 's/^/    /' >> $INT_NAME
            echo "}" >> $INT_NAME
            #echo "    included $EXT_BASE.wrong"
        fi
    elif [ -n "$INTERNAL" ] ; then
        #echo "Converting internal test case from $INT_NAME"
        csplit --digits=1  --quiet --prefix="$INT_NAME." $INT_NAME "/NEA/0" "{*}"
        mv $INT_NAME.0 $EXT_BASE.aff
        rm -f $EXT_BASE.dic $EXT_BASE.good $EXT_BASE.wrong
        #echo "    to $EXT_BASE.aff"
        for FN in 1 2 3 4 ; do
            if [ ! -e "$INT_NAME.$FN" ] ; then
                continue
            fi
            TYPE=$(head -n 1 "$INT_NAME.$FN" |grep -o -E "(DIC|TESTBADGRAM|TESTGOODWORDS|TESTBADWORDS)")
            case "$TYPE" in
                "TESTBADGRAM")
                    EXT=aff
                    tail -n +2 "$INT_NAME.$FN" | grep -v -E "^ *[}#]" | sed 's/^    //' > "$EXT_BASE.$EXT"
                    ;;
                "DIC")
                    EXT=dic
                    COUNT=$(tail -n +2 "$INT_NAME.$FN" | grep -c -v -E "^ *[}#]")
                    echo $COUNT > "$EXT_BASE.$EXT"
                    tail -n +2 "$INT_NAME.$FN" | grep -v -E "^ *[}#]" | sed 's/^    //' >> "$EXT_BASE.$EXT"
                    ;;
                "TESTGOODWORDS")
                    EXT=good 
                    tail -n +2 "$INT_NAME.$FN" | grep -v -E "^ *[}#]" | sed 's/^    //' > "$EXT_BASE.$EXT"
                    ;;
                "TESTBADWORDS")
                    EXT=wrong
                    tail -n +2 "$INT_NAME.$FN" | grep -v -E "^ *[}#]" | sed 's/^    //' > "$EXT_BASE.$EXT"
                    ;;
            esac            
            #echo "    to $EXT_BASE.$EXT"
            rm "$INT_NAME.$FN"
        done
        if [ ! -f "$EXT_BASE.dic" ] ; then
            echo "1" > $EXT_BASE.dic
            # this is not a word
            echo "thssntwd" >> $EXT_BASE.dic
        fi
        if [ ! -f "$EXT_BASE.good" ] ; then
            echo "" > $EXT_BASE.good
        fi
    fi
    if [ -n "$INT_TEST_CMD" ] ; then
        echo "Executing $INT_TEST_CMD $INT_NAME"
        $INT_TEST_CMD "$INT_NAME"
    fi
    if [ -n "$EXT_TEST_CMD" ] ; then
        echo "Executing $EXT_TEST_CMD $EXT_BASE.dic"
        $EXT_TEST_CMD "$EXT_BASE.dic"
    fi
done
