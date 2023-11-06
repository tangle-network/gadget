#[cfg(test)]
mod tests {
    use gadget_core::job_manager::SendFuture;
    use std::error::Error;
    use std::future::Future;
    use std::pin::Pin;
    use test_gadget::work_manager::TestAsyncProtocolParameters;
    use test_gadget::TestBundle;
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    use ark_bls12_377::Fr;
    use ark_ff::{FftField, PrimeField};
    use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
    use async_trait::async_trait;
    use dist_primitives::{
        channel::MpcSerNet,
        dfft::{d_fft, fft_in_place_rearrange},
        utils::pack::transpose,
    };
    use mpc_net::{MpcNet, MpcNetError, MultiplexedStreamID};
    use secret_sharing::pss::PackedSharingParams;
    use test_gadget::test_network::InMemoryNetwork;

    pub async fn d_fft_test<F: FftField + PrimeField, Net: MpcNet>(
        pp: &PackedSharingParams<F>,
        dom: &Radix2EvaluationDomain<F>,
        net: &Net,
    ) {
        let mbyl: usize = dom.size() / pp.l;
        // We apply FFT on this vector
        // let mut x = vec![F::ONE; cd.m];
        let mut x: Vec<F> = Vec::new();
        for i in 0..dom.size() {
            x.push(F::from(i as u64));
        }

        // Output to test against
        let should_be_output = dom.fft(&x);

        fft_in_place_rearrange(&mut x);
        let mut pcoeff: Vec<Vec<F>> = Vec::new();
        for i in 0..mbyl {
            pcoeff.push(x.iter().skip(i).step_by(mbyl).cloned().collect::<Vec<_>>());
            pp.pack_from_public_in_place(&mut pcoeff[i]);
        }

        let pcoeff_share = pcoeff
            .iter()
            .map(|x| x[net.party_id() as usize])
            .collect::<Vec<_>>();

        // Rearranging x

        let peval_share = d_fft(
            pcoeff_share,
            false,
            1,
            false,
            dom,
            pp,
            net,
            MultiplexedStreamID::One,
        )
        .await
        .unwrap();

        // Send to king who reconstructs and checks the answer
        net.send_to_king(&peval_share, MultiplexedStreamID::One)
            .await
            .unwrap()
            .map(|peval_shares| {
                let peval_shares = transpose(peval_shares);

                let pevals: Vec<F> = peval_shares
                    .into_iter()
                    .flat_map(|x| pp.unpack(x))
                    .rev()
                    .collect();

                if net.is_king() {
                    assert_eq!(should_be_output, pevals);
                }
            });
    }

    pub fn setup_log() {
        let _ = SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .try_init();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dfft() -> Result<(), Box<dyn Error>> {
        setup_log();
        test_gadget::simulate_test(
            5,
            10,
            std::time::Duration::from_millis(1000),
            vec![0],
            async_proto_generator,
        )
        .await
    }

    fn async_proto_generator(
        mut params: TestAsyncProtocolParameters<TestBundle>,
    ) -> Pin<Box<dyn SendFuture<'static, ()>>> {
        Box::pin(async move {
            let pp = PackedSharingParams::<Fr>::new(2);
            let dom = Radix2EvaluationDomain::<Fr>::new(1024).unwrap();
            d_fft_test::<Fr, _>(&pp, &dom, &params.test_bundle.network).await;
        })
    }

    struct TestNetwork {
        net: InMemoryNetwork,
        n_peers: usize,
        party_id: u32,
    }

    #[async_trait]
    impl MpcNet for TestNetwork {
        fn n_parties(&self) -> usize {
            self.n_peers
        }

        fn party_id(&self) -> u32 {
            self.party_id
        }

        fn is_init(&self) -> bool {
            true
        }

        async fn client_send_or_king_receive(
            &self,
            bytes: &[u8],
            sid: MultiplexedStreamID,
        ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<bytes::Bytes>>, MpcNetError>> + Send>>
        {
            if self.is_king() {
            } else {
                self.net.send_to()
            }
        }

        async fn client_receive_or_king_send(
            &self,
            bytes: Option<Vec<bytes::Bytes>>,
            sid: MultiplexedStreamID,
        ) -> Pin<Box<dyn Future<Output = Result<bytes::Bytes, MpcNetError>> + Send>> {
            todo!()
        }
    }
}
