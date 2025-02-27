/* eslint-disable camelcase */

exports.shorthands = undefined;

exports.up = pgm => {
    pgm.createTable('vcu_proxy_settings', {
        id: 'id',
        block_number: {
            type: 'int',
            notNull: true
        },
        hash: {
            type: 'varchar(66)',
            unique: true,
            notNull: true
        },
        accounts: {
            type: 'text',
        },
        signer: {
            type: 'varchar(64)',
            notNull: true
        },
        date: {
            type: 'timestamp',
            notNull: true
        },
        created_at: {
            type: 'timestamp',
            notNull: true,
            default: pgm.func('current_timestamp')
        }
    })
};

exports.down = pgm => {
    pgm.dropTable('vcu_proxy_settings')
};