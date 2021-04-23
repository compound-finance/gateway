contract=${1}
certoraRun.py contracts/${contract}.sol \
	--verify ${contract}:spec/Privileged.spec \
	--solc solc8.1 --settings -t=300,-ignoreViewFunctions,-assumeUnwindCond,-b=3 \
	--cloud --msg "${contract} Privileged"
