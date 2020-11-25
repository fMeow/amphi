use syn::{
    parse::{Parse, ParseStream, Result},
    Attribute, ItemMod, Token,
};

pub struct ItemModRestrict(pub ItemMod);

impl Parse for ItemModRestrict {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let mut lookahead = input.lookahead1();
        if lookahead.peek(Token![unsafe]) {
            let ahead = input.fork();
            ahead.parse::<Token![unsafe]>()?;
            lookahead = ahead.lookahead1();
        }
        if lookahead.peek(Token![pub]) || lookahead.peek(Token![mod]) {
            if lookahead.peek(Token![pub]) {
                let ahead = input.fork();
                ahead.parse::<Token![pub]>()?;
                lookahead = ahead.lookahead1();
            }
            if lookahead.peek(Token![mod]) {
                let mut item: ItemMod = input.parse()?;
                item.attrs = attrs;
                Ok(ItemModRestrict(item))
            } else {
                Err(lookahead.error())
            }
        } else {
            Err(lookahead.error())
        }
    }
}
