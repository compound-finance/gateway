contract=${1}
certoraRun.py contracts/${contract}.sol \
  --verify ${contract}:spec/sanity.spec \
  --solc solc8.1 \
  --settings -t=300 \
  --cloud --msg "${contract} Sanity"
