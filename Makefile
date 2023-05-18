all:
	cargo fmt
	cargo test
	cargo check
	# https://zhauniarovich.com/post/2021/2021-09-pedantic-clippy/#paranoid-clippy
	# -D clippy::restriction is way too "safe"/careful
	# -D clippy::pedantic is also probably too safe
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-D clippy::nursery \
		-D clippy::pedantic \
		-A clippy::cast-possible-truncation \
		-A clippy::cast_precision_loss \
		-A clippy::cast-sign-loss #\
		#-A clippy::too-many-lines \
		#-A clippy::missing-panics-doc \
		#-A clippy::missing-errors-doc \
		#-A clippy::len-without-is-empty
