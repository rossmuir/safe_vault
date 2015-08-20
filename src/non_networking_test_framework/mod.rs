// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

#![allow(unsafe_code, unused)] // TODO Remove the unused attribute later

use std::io::{Read, Write};
use sodiumoxide::crypto;

use routing::authority::Authority;
use routing::data::{Data, DataRequest};
use routing::event::Event;
use routing::immutable_data::ImmutableDataType;
use routing::{ExternalRequest, ExternalResponse, NameType};
use routing::error::{RoutingError, InterfaceError, ResponseError};


pub struct MockRouting {
    sender: ::std::sync::mpsc::Sender<Event>,
    client_sender: ::std::sync::mpsc::Sender<Data>,  // for testing only
    network_delay_ms: u32,  // for testing only
}

impl MockRouting {
    pub fn new(event_sender: ::std::sync::mpsc::Sender<(Event)>) -> MockRouting {
        let (client_sender, _) = ::std::sync::mpsc::channel();

        let mock_routing = MockRouting {
            sender: event_sender,
            client_sender: client_sender,
            network_delay_ms: 1000,
        };

        mock_routing
    }

    #[allow(dead_code)]
    pub fn set_network_delay_for_delay_simulation(&mut self, delay_ms: u32) {
        self.network_delay_ms = delay_ms;
    }

    // -----------  the following methods are for testing purpose only   ------------- //
    pub fn client_get(&mut self, client_address: NameType, client_pub_key: crypto::sign::PublicKey,
                      name: NameType) -> ::std::sync::mpsc::Receiver<Data> {
        let cloned_sender = self.sender.clone();
        let _ = ::std::thread::spawn(move || {
            let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Get(DataRequest::ImmutableData(name, ImmutableDataType::Normal)),
                                                       our_authority: Authority::NaeManager(name),
                                                       from_authority: Authority::Client(client_address, client_pub_key),
                                                       response_token: None });
        });
        let (client_sender, client_receiver) = ::std::sync::mpsc::channel();
        self.client_sender = client_sender;
        client_receiver
    }

    pub fn client_put(&mut self, client_address: NameType,
                      client_pub_key: crypto::sign::PublicKey, data: Data) {
        let delay_ms = self.network_delay_ms;
        let cloned_sender = self.sender.clone();
        let _ = ::std::thread::spawn(move || {
            ::std::thread::sleep_ms(delay_ms);
            let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Put(data),
                                                       our_authority: Authority::ClientManager(client_address),
                                                       from_authority: Authority::Client(client_address, client_pub_key),
                                                       response_token: None });
        });
    }

    pub fn churn_event(&mut self, nodes: Vec<NameType>) {
        let cloned_sender = self.sender.clone();
        let _ = ::std::thread::spawn(move || {
            let _ = cloned_sender.send(Event::Churn(nodes));
        });
    }

    // -----------  the above methods are for testing purpose only   ------------- //

    // -----------  the following methods are expected to be API functions   ------------- //

    pub fn get_response(&self, location       : Authority,
                               data           : Data,
                               data_request   : DataRequest,
                               response_token : Option<::routing::SignedToken>) {
        let delay_ms = self.network_delay_ms;
        let cloned_sender = self.sender.clone();
        let cloned_client_sender = self.client_sender.clone();
        let _ = ::std::thread::spawn(move || {
            match location.clone() {
                Authority::NaeManager(_) => {
                    let _ = cloned_sender.send(Event::Response{ response: ExternalResponse::Get(data.clone(), data_request, response_token),
                                                                our_authority: location,
                                                                from_authority: Authority::ManagedNode(NameType::new([7u8; 64])) });
                },
                Authority::Client(_, _) => {
                    let _ = cloned_client_sender.send(data);
                },
                _ => {}
            }
        });
    }

    pub fn get_request(&self, location: Authority, request_for: DataRequest) -> Result<(), ResponseError> {
        let name = match request_for.clone() {
            DataRequest::StructuredData(name, _) => name,
            DataRequest::ImmutableData(name, _) => name,
            DataRequest::PlainData(_) => panic!("Unexpected"),
        };
        let delay_ms = self.network_delay_ms;
        let cloned_sender = self.sender.clone();

        let _ = ::std::thread::spawn(move || {
            ::std::thread::sleep_ms(delay_ms);
            match location.clone() {
                Authority::ManagedNode(_) => {
                    let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Get(request_for),
                                                               our_authority: Authority::ManagedNode(NameType::new([7u8; 64])),
                                                               from_authority: Authority::NaeManager(name),
                                                               response_token: None });
                },
                _ => {}
            }
        });

        Ok(())
    }

    pub fn put_request(&self, location: Authority, data: Data) -> Result<(), ResponseError> {
        let destination = match location.clone() {
            Authority::ClientManager(dest) => dest,
            Authority::NaeManager(dest) => dest,
            Authority::NodeManager(dest) => dest,
            Authority::ManagedNode(dest) => dest,
            _ => panic!("Unexpected"),
        };
        let delay_ms = self.network_delay_ms;
        let cloned_sender = self.sender.clone();

        let _ = ::std::thread::spawn(move || {
            ::std::thread::sleep_ms(delay_ms);
            match location.clone() {
                Authority::NaeManager(_) => {
                    let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Put(data.clone()),
                                                               our_authority: location,
                                                               from_authority: Authority::ClientManager(NameType::new([7u8; 64])),
                                                               response_token: None });
                },
                Authority::NodeManager(_) => {
                    let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Put(data.clone()),
                                                               our_authority: location,
                                                               from_authority: Authority::NaeManager(data.name()),
                                                               response_token: None });
                },
                Authority::ManagedNode(_) => {
                    let _ = cloned_sender.send(Event::Request{ request: ExternalRequest::Put(data.clone()),
                                                               our_authority: location,
                                                               from_authority: Authority::NodeManager(NameType::new([6u8; 64])),
                                                               response_token: None });
                },
                _ => {}
            }
        });

        Ok(())
    }

    // pub fn post(&mut self, location: NameType, data: Data) -> Result<(), ResponseError> {
    //     let delay_ms = self.network_delay_ms;
    //     let data_store = get_storage();

    //     let mut data_store_mutex_guard = data_store.lock().unwrap();
    //     let success = if data_store_mutex_guard.contains_key(&location) {
    //         match (&data, deserialise(data_store_mutex_guard.get(&location).unwrap())) {
    //             (&Data::StructuredData(ref struct_data_new), Ok(Data::StructuredData(ref struct_data_stored))) => {
    //                 if struct_data_new.get_version() != struct_data_stored.get_version() + 1 {
    //                     false
    //                 } else {
    //                     let mut count = 0usize;
    //                     if struct_data_stored.get_owners().iter().any(|key| { // This is more efficient than filter as it will stop whenever sign count reaches >= 50%
    //                         if struct_data_new.get_signatures().iter().any(|sig| ::sodiumoxide::crypto::sign::verify_detached(sig, &struct_data_new.data_to_sign().unwrap(), key)) {
    //                             count += 1;
    //                         }

    //                         count >= struct_data_stored.get_owners().len() / 2 + struct_data_stored.get_owners().len() % 2
    //                     }) {
    //                         if let Ok(raw_data) = serialise(&data) {
    //                             data_store_mutex_guard.insert(location, raw_data);
    //                             sync_disk_storage(&*data_store_mutex_guard);
    //                             true
    //                         } else {
    //                             false
    //                         }
    //                     } else {
    //                         false
    //                     }
    //                 }
    //             },
    //             _ => false,
    //         }
    //     } else {
    //         false
    //     };

    //     // ::std::thread::spawn(move || {
    //     //     ::std::thread::sleep_ms(delay_ms);
    //     //     if !success { // TODO Check how routing is going to handle POST errors
    //     //     }
    //     // });

    //     Ok(())
    // }

    // pub fn delete(&mut self, location: NameType, data: Data) -> Result<(), ResponseError> {
    //     let delay_ms = self.network_delay_ms;
    //     let data_store = get_storage();

    //     let mut data_store_mutex_guard = data_store.lock().unwrap();
    //     let success = if data_store_mutex_guard.contains_key(&location) {
    //         match (&data, deserialise(data_store_mutex_guard.get(&location).unwrap())) {
    //             (&Data::StructuredData(ref struct_data_new), Ok(Data::StructuredData(ref struct_data_stored))) => {
    //                 if struct_data_new.get_version() != struct_data_stored.get_version() + 1 {
    //                     false
    //                 } else {
    //                     let mut count = 0usize;
    //                     if struct_data_stored.get_owners().iter().any(|key| { // This is more efficient than filter as it will stop whenever sign count reaches >= 50%
    //                         if struct_data_new.get_signatures().iter().any(|sig| ::sodiumoxide::crypto::sign::verify_detached(sig, &struct_data_new.data_to_sign().unwrap(), key)) {
    //                             count += 1;
    //                         }

    //                         count >= struct_data_stored.get_owners().len() / 2 + struct_data_stored.get_owners().len() % 2
    //                     }) {
    //                         let _ = data_store_mutex_guard.remove(&location);
    //                         sync_disk_storage(&*data_store_mutex_guard);
    //                         true
    //                     } else {
    //                         false
    //                     }
    //                 }
    //             },
    //             _ => false,
    //         }
    //     } else {
    //         false
    //     };

    //     // ::std::thread::spawn(move || {
    //     //     ::std::thread::sleep_ms(delay_ms);
    //     //     if !success { // TODO Check how routing is going to handle DELETE errors
    //     //     }
    //     // });

    //     Ok(())
    // }

    pub fn bootstrap(&mut self) -> Result<(), RoutingError> {
        Ok(())
    }

    pub fn close(&self) {
        let _ = self.sender.send(Event::Terminated);
    }
}

// #[cfg(test)]
// mod test {
//     use ::std::error::Error;

//     use super::*;

//     #[test]
//     fn map_serialisation() {
//         let mut map_before = ::std::collections::HashMap::<::routing::NameType, Vec<u8>>::new();
//         map_before.insert(::routing::NameType::new([1; 64]), vec![1; 10]);

//         let vec_before = convert_hashmap_to_vec(&map_before);
//         let serialised_data = eval_result!(mock_routing_types::serialise(&vec_before));

//         let vec_after: Vec<(::routing::NameType, Vec<u8>)> = eval_result!(mock_routing_types::deserialise(&serialised_data));
//         let map_after = convert_vec_to_hashmap(vec_after);
//         assert_eq!(map_before, map_after);
//     }

//     #[test]
//     fn check_put_post_get_delete_for_immutable_data() {
//         let notifier = ::std::sync::Arc::new((::std::sync::Mutex::new(None), ::std::sync::Condvar::new()));
//         let account_packet = ::client::user_account::Account::new(None, None);

//         let id_packet = ::routing::types::Id::with_keys(account_packet.get_maid().public_keys().clone(),
//                                                         account_packet.get_maid().secret_keys().clone());

//         let (routing, receiver) = MockRouting::new(id_packet);
//         let (message_queue, reciever_joiner) = ::client::message_queue::MessageQueue::new(notifier.clone(), receiver);

//         let mock_routing = ::std::sync::Arc::new(::std::sync::Mutex::new(routing));
//         let mock_routing_clone = mock_routing.clone();

//         let mock_routing_stop_flag = ::std::sync::Arc::new(::std::sync::Mutex::new(false));
//         let mock_routing_stop_flag_clone = mock_routing_stop_flag.clone();

//         struct RAIIThreadExit {
//             routing_stop_flag: ::std::sync::Arc<::std::sync::Mutex<bool>>,
//             join_handle: Option<::std::thread::JoinHandle<()>>,
//         }

//         impl Drop for RAIIThreadExit {
//             fn drop(&mut self) {
//                 *self.routing_stop_flag.lock().unwrap() = true;
//                 self.join_handle.take().unwrap().join().unwrap();
//             }
//         }

//         let _managed_thread = RAIIThreadExit {
//             routing_stop_flag: mock_routing_stop_flag,
//             join_handle: Some(::std::thread::spawn(move || {
//                 while !*mock_routing_stop_flag_clone.lock().unwrap() {
//                     ::std::thread::sleep_ms(10);
//                     mock_routing_clone.lock().unwrap().run();
//                 }
//                 mock_routing_clone.lock().unwrap().close();
//                 reciever_joiner.join().unwrap();
//             })),
//         };

//         // Construct ImmutableData
//         let orig_raw_data: Vec<u8> = eval_result!(mock_routing_types::generate_random_vector(100));
//         let orig_immutable_data = ::client::ImmutableData::new(::client::ImmutableDataType::Normal, orig_raw_data.clone());
//         let orig_data = ::client::Data::ImmutableData(orig_immutable_data.clone());

//         // First PUT should succeed
//         {
//             match mock_routing.lock().unwrap().put(orig_immutable_data.name(), orig_data.clone()) {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // GET ImmutableData should pass
//         {
//             let mut mock_routing_guard = mock_routing.lock().unwrap();
//             match mock_routing_guard.get(orig_immutable_data.name(), ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal)) {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              orig_immutable_data.name(),
//                                                                                              ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::ImmutableData(received_immutable_data) => assert_eq!(orig_immutable_data, received_immutable_data),
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         // Subsequent PUTs for same ImmutableData should succeed - De-duplication
//         {
//             let put_result = mock_routing.lock().unwrap().put(orig_immutable_data.name(), orig_data.clone());
//             match put_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // Construct Backup ImmutableData
//         let new_immutable_data = ::client::ImmutableData::new(::client::ImmutableDataType::Backup, orig_raw_data);
//         let new_data = ::client::Data::ImmutableData(new_immutable_data.clone());

//         // Subsequent PUTs for same ImmutableData of different type should fail
//         {
//             let put_result = mock_routing.lock().unwrap().put(orig_immutable_data.name(), new_data);
//             match put_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // POSTs for ImmutableData should fail
//         {
//             let post_result = mock_routing.lock().unwrap().post(orig_immutable_data.name(), orig_data.clone());
//             match post_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in POST !!"),
//             }
//         }

//         // DELETEs of ImmutableData should fail
//         {
//             let delete_result = mock_routing.lock().unwrap().delete(orig_immutable_data.name(), orig_data);
//             match delete_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in DELETE !!"),
//             }
//         }

//         // GET ImmutableData should pass
//         {
//             let mut mock_routing_mutex_guard = mock_routing.lock().unwrap();
//             match mock_routing_mutex_guard.get(orig_immutable_data.name(), ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal)) {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              orig_immutable_data.name(),
//                                                                                              ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::ImmutableData(received_immutable_data) => assert_eq!(orig_immutable_data, received_immutable_data), // TODO Improve by directly assert_eq!(data)
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }
//     }

//     #[test]
//     fn check_put_post_get_delete_for_structured_data() {
//         let notifier = ::std::sync::Arc::new((::std::sync::Mutex::new(None), ::std::sync::Condvar::new()));
//         let account_packet = ::client::user_account::Account::new(None, None);

//         let id_packet = ::routing::types::Id::with_keys(account_packet.get_maid().public_keys().clone(),
//                                                       account_packet.get_maid().secret_keys().clone());

//         let (routing, receiver) = MockRouting::new(id_packet);
//         let (message_queue, receiver_joiner) = ::client::message_queue::MessageQueue::new(notifier.clone(), receiver);
//         let mock_routing = ::std::sync::Arc::new(::std::sync::Mutex::new(routing));
//         let mock_routing_clone = mock_routing.clone();

//         let mock_routing_stop_flag = ::std::sync::Arc::new(::std::sync::Mutex::new(false));
//         let mock_routing_stop_flag_clone = mock_routing_stop_flag.clone();

//         struct RAIIThreadExit {
//             routing_stop_flag: ::std::sync::Arc<::std::sync::Mutex<bool>>,
//             join_handle: Option<::std::thread::JoinHandle<()>>,
//         }

//         impl Drop for RAIIThreadExit {
//             fn drop(&mut self) {
//                 *self.routing_stop_flag.lock().unwrap() = true;
//                 self.join_handle.take().unwrap().join().unwrap();
//             }
//         }

//         let _managed_thread = RAIIThreadExit {
//             routing_stop_flag: mock_routing_stop_flag,
//             join_handle: Some(::std::thread::spawn(move || {
//                 while !*mock_routing_stop_flag_clone.lock().unwrap() {
//                     ::std::thread::sleep_ms(10);
//                     mock_routing_clone.lock().unwrap().run();
//                 }
//                 mock_routing_clone.lock().unwrap().close();
//                 receiver_joiner.join().unwrap();
//             })),
//         };

//         // Construct ImmutableData
//         let orig_data: Vec<u8> = eval_result!(mock_routing_types::generate_random_vector(100));
//         let orig_immutable_data = ::client::ImmutableData::new(::client::ImmutableDataType::Normal, orig_data);
//         let orig_data_immutable = ::client::Data::ImmutableData(orig_immutable_data.clone());

//         // Construct StructuredData, 1st version, for this ImmutableData
//         const TYPE_TAG: u64 = 999;
//         let keyword = eval_result!(mock_routing_types::generate_random_string(10));
//         let pin = mock_routing_types::generate_random_pin();
//         let user_id = ::client::user_account::Account::generate_network_id(&keyword, pin);
//         let mut account_version = ::client::StructuredData::new(TYPE_TAG,
//                                                                 user_id.clone(),
//                                                                 0,
//                                                                 eval_result!(mock_routing_types::serialise(&vec![orig_immutable_data.name()])),
//                                                                 vec![account_packet.get_public_maid().public_keys().0.clone()],
//                                                                 Vec::new(),
//                                                                 &account_packet.get_maid().secret_keys().0);
//         let mut data_account_version = ::client::Data::StructuredData(account_version.clone());

//         // First PUT of StructuredData should succeed
//         {
//             match mock_routing.lock().unwrap().put(account_version.name(), data_account_version.clone()) {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // PUT for ImmutableData should succeed
//         {
//             match mock_routing.lock().unwrap().put(orig_immutable_data.name(), orig_data_immutable.clone()) {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         let mut received_structured_data: ::client::StructuredData;

//         // GET StructuredData should pass
//         {
//             match mock_routing.lock().unwrap().get(account_version.name(), ::client::DataRequest::StructuredData(TYPE_TAG)) {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              account_version.name(),
//                                                                                              ::client::DataRequest::StructuredData(TYPE_TAG));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::StructuredData(struct_data) => {
//                                     received_structured_data = struct_data;
//                                     assert!(account_version == received_structured_data);
//                                 },
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         // GET ImmutableData from lastest version of StructuredData should pass
//         {
//             let mut location_vec = eval_result!(mock_routing_types::deserialise::<Vec<::routing::NameType>>(received_structured_data.get_data()));
//             match mock_routing.lock().unwrap().get(eval_option!(location_vec.pop(), "Value must exist !"), ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal)) {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              orig_immutable_data.name(),
//                                                                                              ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::ImmutableData(received_immutable_data) => assert_eq!(orig_immutable_data, received_immutable_data),
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         // Construct ImmutableData
//         let new_data: Vec<u8> = eval_result!(mock_routing_types::generate_random_vector(100));
//         let new_immutable_data = ::client::ImmutableData::new(::client::ImmutableDataType::Normal, new_data);
//         let new_data_immutable = ::client::Data::ImmutableData(new_immutable_data.clone());

//         // PUT for new ImmutableData should succeed
//         {
//             match mock_routing.lock().unwrap().put(new_immutable_data.name(), new_data_immutable) {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // Construct StructuredData, 2nd version, for this ImmutableData - IVALID Versioning
//         let invalid_version_account_version = ::client::StructuredData::new(TYPE_TAG,
//                                                                             user_id.clone(),
//                                                                             0,
//                                                                             mock_routing_types::serialise(&vec![orig_immutable_data.name(), new_immutable_data.name()]).ok().unwrap(),
//                                                                             vec![account_packet.get_public_maid().public_keys().0.clone()],
//                                                                             Vec::new(),
//                                                                             &account_packet.get_maid().secret_keys().0);
//         let invalid_version_data_account_version = ::client::Data::StructuredData(invalid_version_account_version.clone());

//         // Construct StructuredData, 2nd version, for this ImmutableData - IVALID Signature
//         let invalid_signature_account_version = ::client::StructuredData::new(TYPE_TAG,
//                                                                               user_id.clone(),
//                                                                               1,
//                                                                               mock_routing_types::serialise(&vec![orig_immutable_data.name(), new_immutable_data.name()]).ok().unwrap(),
//                                                                               vec![account_packet.get_public_maid().public_keys().0.clone()],
//                                                                               Vec::new(),
//                                                                               &account_packet.get_mpid().secret_keys().0);
//         let invalid_signature_data_account_version = ::client::Data::StructuredData(invalid_signature_account_version.clone());

//         // Construct StructuredData, 2nd version, for this ImmutableData - Valid
//         account_version = ::client::StructuredData::new(TYPE_TAG,
//                                                         user_id.clone(),
//                                                         1,
//                                                         mock_routing_types::serialise(&vec![orig_immutable_data.name(), new_immutable_data.name()]).ok().unwrap(),
//                                                         vec![account_packet.get_public_maid().public_keys().0.clone()],
//                                                         Vec::new(),
//                                                         &account_packet.get_maid().secret_keys().0);
//         data_account_version = ::client::Data::StructuredData(account_version.clone());

//         // Subsequent PUTs for same StructuredData should fail
//         {
//             let put_result = mock_routing.lock().unwrap().put(account_version.name(), data_account_version.clone());
//             match put_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in PUT !!"),
//             }
//         }

//         // Subsequent POSTSs for same StructuredData should fail if versioning is invalid
//         {
//             let post_result = mock_routing.lock().unwrap().post(invalid_version_account_version.name(), invalid_version_data_account_version.clone());
//             match post_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in POST !!"),
//             }
//         }

//         // Subsequent POSTSs for same StructuredData should fail if signature is invalid
//         {
//             let post_result = mock_routing.lock().unwrap().post(invalid_signature_account_version.name(), invalid_signature_data_account_version.clone());
//             match post_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in POST !!"),
//             }
//         }

//         // Subsequent POSTSs for existing StructuredData version should pass for valid update
//         {
//             let post_result = mock_routing.lock().unwrap().post(account_version.name(), data_account_version.clone());
//             match post_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in POST !!"),
//             }
//         }

//         // GET for new StructuredData version should pass
//         {
//             match mock_routing.lock().unwrap().get(account_version.name(), ::client::DataRequest::StructuredData(TYPE_TAG)) {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              account_version.name(),
//                                                                                              ::client::DataRequest::StructuredData(TYPE_TAG));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::StructuredData(structured_data) => {
//                                     received_structured_data = structured_data;
//                                     assert!(received_structured_data == account_version);
//                                 },
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         let location_vec = eval_result!(mock_routing_types::deserialise::<Vec<::routing::NameType>>(received_structured_data.get_data()));
//         assert_eq!(location_vec.len(), 2);

//         // GET new ImmutableData should pass
//         {
//             let get_result = mock_routing.lock().unwrap().get(location_vec[1].clone(), ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//             match get_result {
//                 Ok(()) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              location_vec[1].clone(),
//                                                                                              ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::ImmutableData(received_immutable_data) => assert_eq!(new_immutable_data, received_immutable_data),
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         // GET original ImmutableData should pass
//         {
//             let get_result = mock_routing.lock().unwrap().get(location_vec[0].clone(), ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//             match get_result {
//                 Ok(id) => {
//                     let mut response_getter = ::client::response_getter::ResponseGetter::new(Some(notifier.clone()),
//                                                                                              message_queue.clone(),
//                                                                                              location_vec[0].clone(),
//                                                                                              ::client::DataRequest::ImmutableData(::client::ImmutableDataType::Normal));
//                     match response_getter.get() {
//                         Ok(data) => {
//                             match data {
//                                 ::client::Data::ImmutableData(received_immutable_data) => assert_eq!(orig_immutable_data, received_immutable_data),
//                                 _ => panic!("Unexpected!"),
//                             }
//                         },
//                         Err(_) => panic!("Should have found data put before by a PUT"),
//                     }
//                 },
//                 Err(_) => panic!("Failure in GET !!"),
//             }
//         }

//         // TODO this will not function properly presently .. DELETE needs a version Bump too
//         // DELETE of Structured Data should succeed
//         {
//             let delete_result = mock_routing.lock().unwrap().delete(account_version.name(), data_account_version.clone());
//             match delete_result {
//                 Ok(()) => (),
//                 Err(_) => panic!("Failure in DELETE !!"),
//             }
//         }
//     }
// }