CREATE TABLE signatures (
    auction_id bigint NOT NULL,
    user_address bytea NOT NULL,
    signature bytea NOT NULL, -- r + s + v
    PRIMARY KEY (auction_id, user_address)
);

CREATE INDEX user_address ON signatures USING HASH (user_address);

CREATE INDEX auction_id ON signatures USING BTREE (auction_id);