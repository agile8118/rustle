CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_boards_owner_id ON boards(owner_id);
CREATE INDEX idx_columns_board_id_position ON board_columns(board_id, position);
CREATE INDEX idx_cards_column_id_position ON cards(column_id, position);
CREATE INDEX idx_comments_card_id_created_at ON comments(card_id, created_at);
CREATE INDEX idx_labels_owner_id ON labels(owner_id);
