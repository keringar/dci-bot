#!/bin/bash
cd dci-bot
cargo b --release
if [ "$?" -eq 0 ]; then
	cd
	DCI_PASSWORD=w73Z23yVN2d@jL%6G* DCI_SECRET=FeRDoJkNdIqK_SVUji95TpXLd_I ./dci-bot/target/release/dci-bot
fi
