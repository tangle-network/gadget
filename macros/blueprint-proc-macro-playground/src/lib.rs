use gadget_blueprint_proc_macro::job;

#[job(id = 0, params(n, t), result(_))]
pub fn keygen(n: u16, t: u8) -> Vec<u8> {
    let _ = (n, t);
    vec![0; 33]
}

#[cfg(test)]
mod tests {

    #[test]
    fn generated_blueprint() {
        eprintln!("{}", super::KEYGEN_JOB_DEF);
        assert_eq!(super::KEYGEN_JOB_ID, 0);
    }
}
