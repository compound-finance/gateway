* make a digital ocean account, get your access token and upload a ssh key
* export DO_PAT="YOUR_PERSONAL_ACCESS_TOKEN"

* `ssh-add -L`
* terraform apply -var "do_token=${DO_PAT}"
    * will return the instance ip
* ssh root@{IP_ADDR}
    * git clone https://github.com/compound-finance/gateway.git
    * cd gateway
    * ./scripts/get_substrate.sh
    * source ~/.cargo/env
    * ./scripts/benchmark.sh
* sftp root@{IP_ADDR}
    * get gateway/pallets/cash/src/weights.rs
